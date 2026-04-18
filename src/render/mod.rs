pub mod activity;
pub mod class;
pub mod mindmap;
pub mod primitives;
pub mod sequence;
pub mod state;
pub mod theme;
pub mod timing;
pub mod usecase;

use crate::ast::DiagramAst;
use crate::layout;
use svg::Document;
use theme::Theme;

pub fn render(ast: &DiagramAst) -> Document {
    match ast {
        DiagramAst::Sequence(seq) => {
            let theme = build_theme(&seq.skinparams);
            let layout = layout::sequence::layout(seq);
            sequence::render(&layout, &theme)
        }
        DiagramAst::Class(cls) => {
            let theme = build_theme(&cls.skinparams);
            let layout = layout::class::layout(cls);
            class::render(&layout, &theme)
        }
        DiagramAst::Activity(act) => {
            let theme = build_theme(&act.skinparams);
            let layout = layout::activity::layout(act);
            activity::render(&layout, &theme)
        }
        DiagramAst::State(st) => {
            let theme = build_theme(&st.skinparams);
            let layout = layout::state::layout(st);
            state::render(&layout, &theme)
        }
        DiagramAst::UseCase(uc) => {
            let theme = build_theme(&uc.skinparams);
            let layout = layout::usecase::layout(uc);
            usecase::render(&layout, &theme)
        }
        DiagramAst::Timing(tm) => {
            let theme = build_theme(&tm.skinparams);
            let layout = layout::timing::layout(tm);
            timing::render(&layout, &theme)
        }
        DiagramAst::MindMap(mm) => {
            let theme = build_theme(&mm.skinparams);
            let layout = layout::mindmap::layout(mm);
            mindmap::render(&layout, &theme)
        }
    }
}

fn build_theme(skinparams: &[(String, String)]) -> Theme {
    let mut theme = Theme::default();
    theme.apply_all(skinparams.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    theme
}
