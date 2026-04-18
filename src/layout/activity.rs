use crate::ast::activity::*;

// Layout constants
const ACTION_W: f64 = 160.0;
const ACTION_H: f64 = 36.0;
const DIAMOND_SIZE: f64 = 28.0; // half-width/height of diamond
const CIRCLE_R: f64 = 14.0;
const V_GAP: f64 = 30.0;
const H_GAP: f64 = 50.0;
#[allow(dead_code)]
const BRANCH_MIN_W: f64 = 140.0;
const SIDE_MARGIN: f64 = 40.0;
const TOP_MARGIN: f64 = 30.0;
const TITLE_H: f64 = 30.0;
const FONT_CHAR_W: f64 = 7.0;

#[derive(Debug, Clone)]
pub enum Shape {
    StartEnd, // filled circle
    Action,   // rounded rectangle
    Decision, // diamond
    MergeBar, // thick horizontal bar (fork join)
    Note,
    Arrow,
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: usize,
    pub x: f64, // center x
    pub y: f64, // center y (for circles/diamonds) or top-left y for rects
    pub w: f64,
    pub h: f64,
    pub shape: Shape,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub dashed: bool,
}

pub struct ActivityLayout {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub total_width: f64,
    pub total_height: f64,
    pub title: Option<String>,
}

struct Builder {
    nodes: Vec<LayoutNode>,
    edges: Vec<LayoutEdge>,
    next_id: usize,
    cx: f64, // current center-x of main lane
    y: f64,  // current top y
    total_width: f64,
}

impl Builder {
    fn new(cx: f64) -> Self {
        Builder {
            nodes: Vec::new(),
            edges: Vec::new(),
            next_id: 0,
            cx,
            y: 0.0,
            total_width: cx * 2.0,
        }
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn push_node(
        &mut self,
        shape: Shape,
        label: impl Into<String>,
        w: f64,
        h: f64,
        color: Option<String>,
    ) -> usize {
        let id = self.alloc_id();
        let x = self.cx - w / 2.0;
        self.nodes.push(LayoutNode {
            id,
            x,
            y: self.y,
            w,
            h,
            shape,
            label: label.into(),
            color,
        });
        self.total_width = self.total_width.max(x + w + SIDE_MARGIN);
        id
    }

    fn connect(&mut self, from: usize, to: usize, label: Option<String>, dashed: bool) {
        self.edges.push(LayoutEdge {
            from,
            to,
            label,
            dashed,
        });
    }

    fn advance(&mut self, h: f64) {
        self.y += h + V_GAP;
    }

    /// Lay out a list of elements sequentially, returning the id of the last node placed.
    fn layout_seq(&mut self, elements: &[ActivityElement], prev: Option<usize>) -> Option<usize> {
        let mut cur = prev;
        for elem in elements {
            let next = self.layout_one(elem, cur);
            cur = next.or(cur);
        }
        cur
    }

    fn layout_one(&mut self, elem: &ActivityElement, prev: Option<usize>) -> Option<usize> {
        match elem {
            ActivityElement::Start => {
                let id = self.push_node(Shape::StartEnd, "", CIRCLE_R * 2.0, CIRCLE_R * 2.0, None);
                self.advance(CIRCLE_R * 2.0);
                if let Some(p) = prev {
                    self.connect(p, id, None, false);
                }
                Some(id)
            }
            ActivityElement::Stop
            | ActivityElement::End
            | ActivityElement::Kill
            | ActivityElement::Detach => {
                let id = self.push_node(
                    Shape::StartEnd,
                    "stop",
                    CIRCLE_R * 2.0,
                    CIRCLE_R * 2.0,
                    None,
                );
                self.advance(CIRCLE_R * 2.0);
                if let Some(p) = prev {
                    self.connect(p, id, None, false);
                }
                Some(id)
            }
            ActivityElement::Action(a) => {
                let w = label_width(&a.label).max(ACTION_W);
                let id = self.push_node(Shape::Action, &a.label, w, ACTION_H, a.color.clone());
                self.advance(ACTION_H);
                if let Some(p) = prev {
                    self.connect(p, id, None, false);
                }
                Some(id)
            }
            ActivityElement::Arrow(lbl) => {
                // Arrows between actions — just tag the edge
                if let (Some(p), Some(lbl)) = (prev, lbl) {
                    if let Some(last_edge) = self.edges.iter_mut().rev().find(|e| e.from == p) {
                        last_edge.label = Some(lbl.clone());
                    }
                }
                prev
            }
            ActivityElement::If(block) => Some(self.layout_if(block, prev)),
            ActivityElement::While(block) => Some(self.layout_while(block, prev)),
            ActivityElement::Repeat(block) => Some(self.layout_repeat(block, prev)),
            ActivityElement::Fork(block) => Some(self.layout_fork(block, prev)),
            ActivityElement::Partition(p) => self.layout_seq(&p.body, prev),
            ActivityElement::Note(n) => {
                let id = self.push_node(Shape::Note, &n.text, ACTION_W, ACTION_H, None);
                self.advance(ACTION_H);
                Some(id)
            }
            ActivityElement::Label(_) => prev, // swim-lane labels: skip for now
        }
    }

    fn layout_if(&mut self, block: &IfBlock, prev: Option<usize>) -> usize {
        // Diamond decision node
        let d_size = DIAMOND_SIZE * 2.0;
        let decision_id = self.push_node(Shape::Decision, &block.condition, d_size, d_size, None);
        self.advance(d_size);
        if let Some(p) = prev {
            self.connect(p, decision_id, None, false);
        }

        let merge_id = self.alloc_id();
        let total_branches = 1 + block.elseif_branches.len() + 1;
        let branch_w = ACTION_W + H_GAP;
        let total_w = branch_w * total_branches as f64;
        let start_x = self.cx - total_w / 2.0 + branch_w / 2.0;

        let saved_y = self.y;
        let mut branch_ends: Vec<(usize, Option<String>)> = Vec::new();
        let mut max_y: f64 = self.y;

        // Then branch
        {
            let saved_cx = self.cx;
            self.cx = start_x;
            self.y = saved_y;
            let lbl = block.then_label.clone();
            let end = self.layout_seq(&block.then_body, Some(decision_id));
            branch_ends.push((end.unwrap_or(decision_id), lbl));
            max_y = max_y.max(self.y);
            self.cx = saved_cx;
        }

        // Elseif branches
        for (i, branch) in block.elseif_branches.iter().enumerate() {
            let saved_cx = self.cx;
            self.cx = start_x + branch_w * (1 + i) as f64;
            self.y = saved_y;
            let lbl = branch.label.clone();
            let end = self.layout_seq(&branch.body, Some(decision_id));
            branch_ends.push((end.unwrap_or(decision_id), lbl));
            max_y = max_y.max(self.y);
            self.cx = saved_cx;
        }

        // Else branch
        {
            let saved_cx = self.cx;
            self.cx = start_x + branch_w * (total_branches - 1) as f64;
            self.y = saved_y;
            let lbl = block.else_label.clone();
            let end = self.layout_seq(&block.else_body, Some(decision_id));
            branch_ends.push((end.unwrap_or(decision_id), lbl));
            max_y = max_y.max(self.y);
            self.cx = saved_cx;
        }

        // Merge node
        self.y = max_y;
        let merge_x = self.cx - DIAMOND_SIZE;
        self.nodes.push(LayoutNode {
            id: merge_id,
            x: merge_x,
            y: self.y,
            w: DIAMOND_SIZE * 2.0,
            h: DIAMOND_SIZE * 2.0,
            shape: Shape::Decision,
            label: String::new(),
            color: None,
        });
        self.advance(DIAMOND_SIZE * 2.0);

        for (end_id, lbl) in branch_ends {
            self.connect(end_id, merge_id, lbl, false);
        }
        self.total_width = self.total_width.max(start_x + total_w + SIDE_MARGIN);
        merge_id
    }

    fn layout_while(&mut self, block: &WhileBlock, prev: Option<usize>) -> usize {
        let d_size = DIAMOND_SIZE * 2.0;
        let cond_id = self.push_node(Shape::Decision, &block.condition, d_size, d_size, None);
        self.advance(d_size);
        if let Some(p) = prev {
            self.connect(p, cond_id, None, false);
        }

        let body_end = self.layout_seq(&block.body, Some(cond_id));

        // Back-edge from body end to condition
        if let Some(end_id) = body_end {
            self.connect(end_id, cond_id, None, true);
        }

        // Exit node placeholder — merge after loop
        let exit_id = self.push_node(
            Shape::Arrow,
            block.exit_label.clone().unwrap_or_default(),
            4.0,
            4.0,
            None,
        );
        self.connect(cond_id, exit_id, block.exit_label.clone(), false);
        exit_id
    }

    fn layout_repeat(&mut self, block: &RepeatBlock, prev: Option<usize>) -> usize {
        let body_start_y = self.y;
        let first = self.layout_seq(&block.body, prev);

        let d_size = DIAMOND_SIZE * 2.0;
        let cond_id = self.push_node(Shape::Decision, &block.condition, d_size, d_size, None);
        self.advance(d_size);
        if let Some(p) = first {
            self.connect(p, cond_id, None, false);
        }

        // Back-edge: "yes" goes back up to first body node
        // We approximate: connect cond back to a virtual point at body_start_y
        // In SVG this becomes a curved back-arrow
        let back_id = self.push_node(Shape::Arrow, String::new(), 4.0, 4.0, None);
        self.nodes.last_mut().unwrap().y = body_start_y;
        self.connect(cond_id, back_id, block.label.clone(), true);

        // Exit
        let exit_id = self.push_node(Shape::Arrow, String::new(), 4.0, 4.0, None);
        self.connect(cond_id, exit_id, Some("no".to_string()), false);
        exit_id
    }

    fn layout_fork(&mut self, block: &ForkBlock, prev: Option<usize>) -> usize {
        let bar_w = (block.branches.len() as f64) * (ACTION_W + H_GAP) + H_GAP;
        let bar_h = 8.0;
        let fork_id = self.push_node(Shape::MergeBar, "", bar_w, bar_h, None);
        self.advance(bar_h);
        if let Some(p) = prev {
            self.connect(p, fork_id, None, false);
        }

        let branch_w = ACTION_W + H_GAP;
        let start_x = self.cx - bar_w / 2.0 + branch_w / 2.0;
        let saved_y = self.y;
        let mut branch_ends: Vec<usize> = Vec::new();
        let mut max_y: f64 = self.y;

        for (i, branch) in block.branches.iter().enumerate() {
            let saved_cx = self.cx;
            self.cx = start_x + branch_w * i as f64;
            self.y = saved_y;
            let end = self.layout_seq(branch, Some(fork_id));
            branch_ends.push(end.unwrap_or(fork_id));
            max_y = max_y.max(self.y);
            self.cx = saved_cx;
        }

        self.y = max_y;
        let join_id = self.push_node(Shape::MergeBar, "", bar_w, bar_h, None);
        self.advance(bar_h);
        for end_id in branch_ends {
            self.connect(end_id, join_id, None, false);
        }
        self.total_width = self.total_width.max(start_x + bar_w + SIDE_MARGIN);
        join_id
    }
}

fn label_width(s: &str) -> f64 {
    s.len() as f64 * FONT_CHAR_W + 20.0
}

pub fn layout(diagram: &ActivityDiagram) -> ActivityLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_H
    } else {
        0.0
    };
    let initial_cx = SIDE_MARGIN + ACTION_W / 2.0 + 20.0;
    let mut b = Builder::new(initial_cx);
    b.y = TOP_MARGIN + title_off;
    b.total_width = initial_cx * 2.0 + SIDE_MARGIN;

    b.layout_seq(&diagram.elements, None);

    // If any node was placed with a negative left edge (e.g. wide if/else
    // branches positioned around the main lane), shift everything right so the
    // whole diagram sits inside the canvas.
    let min_x = b.nodes.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
    if min_x.is_finite() && min_x < SIDE_MARGIN {
        let shift = SIDE_MARGIN - min_x;
        for n in &mut b.nodes {
            n.x += shift;
        }
        b.total_width += shift;
    }

    let max_right = b.nodes.iter().map(|n| n.x + n.w).fold(0.0_f64, f64::max);
    let total_width = b
        .total_width
        .max(max_right + SIDE_MARGIN)
        .max(initial_cx * 2.0);

    let total_height = b.y + SIDE_MARGIN;
    ActivityLayout {
        nodes: b.nodes,
        edges: b.edges,
        total_width,
        total_height,
        title: diagram.title.clone(),
    }
}
