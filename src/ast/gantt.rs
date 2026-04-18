#[derive(Debug, Clone)]
pub struct GanttTask {
    pub name: String,
    pub duration: u32, // in days
    pub milestone: bool,
    pub depends_on: Option<String>, // start after the named task's end
    pub fixed_start: Option<u32>,   // explicit start day (wins over depends_on)
}

#[derive(Debug, Clone, Default)]
pub struct GanttDiagram {
    pub title: Option<String>,
    pub tasks: Vec<GanttTask>,
    pub skinparams: Vec<(String, String)>,
}

impl GanttTask {
    pub fn new(name: String) -> Self {
        GanttTask {
            name,
            duration: 1,
            milestone: false,
            depends_on: None,
            fixed_start: None,
        }
    }
}
