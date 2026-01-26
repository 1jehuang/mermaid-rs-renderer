use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TopDown,
    LeftRight,
    BottomTop,
    RightLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramKind {
    Flowchart,
    Class,
    State,
    Sequence,
    Er,
    Pie,
    Mindmap,
    Journey,
    Timeline,
    Gantt,
    Requirement,
    GitGraph,
    C4,
    Sankey,
    Quadrant,
    ZenUML,
    Block,
    Packet,
    Kanban,
    Architecture,
    Radar,
    Treemap,
    XYChart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceFrameKind {
    Alt,
    Opt,
    Loop,
    Par,
    Rect,
    Critical,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceNotePosition {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateNotePosition {
    LeftOf,
    RightOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceActivationKind {
    Activate,
    Deactivate,
}

#[derive(Debug, Clone)]
pub struct SequenceActivation {
    pub participant: String,
    pub index: usize,
    pub kind: SequenceActivationKind,
}

#[derive(Debug, Clone)]
pub struct SequenceNote {
    pub position: SequenceNotePosition,
    pub participants: Vec<String>,
    pub label: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct QuadrantPoint {
    pub label: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Default)]
pub struct QuadrantData {
    pub title: Option<String>,
    pub x_axis_left: Option<String>,
    pub x_axis_right: Option<String>,
    pub y_axis_bottom: Option<String>,
    pub y_axis_top: Option<String>,
    pub quadrant_labels: [Option<String>; 4], // top-right, top-left, bottom-left, bottom-right
    pub points: Vec<QuadrantPoint>,
}

#[derive(Debug, Clone)]
pub struct GanttTask {
    pub id: String,
    pub label: String,
    pub start: Option<String>,
    pub duration: Option<String>,
    pub after: Option<String>,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitGraphCommitType {
    Normal,
    Reverse,
    Highlight,
    Merge,
    CherryPick,
}

#[derive(Debug, Clone)]
pub struct GitGraphCommit {
    pub id: String,
    pub message: Option<String>,
    pub seq: usize,
    pub commit_type: GitGraphCommitType,
    pub custom_type: Option<GitGraphCommitType>,
    pub tags: Vec<String>,
    pub parents: Vec<String>,
    pub branch: String,
    pub custom_id: bool,
}

#[derive(Debug, Clone)]
pub struct GitGraphBranch {
    pub name: String,
    pub order: Option<f32>,
    pub insertion_index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GitGraphData {
    pub main_branch: String,
    pub commits: Vec<GitGraphCommit>,
    pub branches: Vec<GitGraphBranch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum C4ShapeKind {
    Person,
    ExternalPerson,
    System,
    SystemDb,
    SystemQueue,
    ExternalSystem,
    ExternalSystemDb,
    ExternalSystemQueue,
    Container,
    ContainerDb,
    ContainerQueue,
    ExternalContainer,
    ExternalContainerDb,
    ExternalContainerQueue,
    Component,
    ComponentDb,
    ComponentQueue,
    ExternalComponent,
    ExternalComponentDb,
    ExternalComponentQueue,
}

impl C4ShapeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            C4ShapeKind::Person => "person",
            C4ShapeKind::ExternalPerson => "external_person",
            C4ShapeKind::System => "system",
            C4ShapeKind::SystemDb => "system_db",
            C4ShapeKind::SystemQueue => "system_queue",
            C4ShapeKind::ExternalSystem => "external_system",
            C4ShapeKind::ExternalSystemDb => "external_system_db",
            C4ShapeKind::ExternalSystemQueue => "external_system_queue",
            C4ShapeKind::Container => "container",
            C4ShapeKind::ContainerDb => "container_db",
            C4ShapeKind::ContainerQueue => "container_queue",
            C4ShapeKind::ExternalContainer => "external_container",
            C4ShapeKind::ExternalContainerDb => "external_container_db",
            C4ShapeKind::ExternalContainerQueue => "external_container_queue",
            C4ShapeKind::Component => "component",
            C4ShapeKind::ComponentDb => "component_db",
            C4ShapeKind::ComponentQueue => "component_queue",
            C4ShapeKind::ExternalComponent => "external_component",
            C4ShapeKind::ExternalComponentDb => "external_component_db",
            C4ShapeKind::ExternalComponentQueue => "external_component_queue",
        }
    }
}

#[derive(Debug, Clone)]
pub struct C4Shape {
    pub id: String,
    pub label: String,
    pub type_label: Option<String>,
    pub techn: Option<String>,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub parent_boundary: String,
    pub kind: C4ShapeKind,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct C4Boundary {
    pub id: String,
    pub label: String,
    pub boundary_type: String,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub parent_boundary: String,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4RelKind {
    Rel,
    BiRel,
    RelUp,
    RelDown,
    RelLeft,
    RelRight,
    RelBack,
}

#[derive(Debug, Clone)]
pub struct C4Rel {
    pub kind: C4RelKind,
    pub from: String,
    pub to: String,
    pub label: String,
    pub techn: Option<String>,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub offset_x: f32,
    pub offset_y: f32,
    pub line_color: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct C4Data {
    pub shapes: Vec<C4Shape>,
    pub boundaries: Vec<C4Boundary>,
    pub rels: Vec<C4Rel>,
    pub c4_type: Option<String>,
    pub c4_shape_in_row_override: Option<usize>,
    pub c4_boundary_in_row_override: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SequenceBox {
    pub label: Option<String>,
    pub color: Option<String>,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StateNote {
    pub position: StateNotePosition,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct SequenceFrameSection {
    pub label: Option<String>,
    pub start_idx: usize,
    pub end_idx: usize,
}

#[derive(Debug, Clone)]
pub struct SequenceFrame {
    pub kind: SequenceFrameKind,
    pub sections: Vec<SequenceFrameSection>,
    pub start_idx: usize,
    pub end_idx: usize,
}

impl Direction {
    pub fn from_token(token: &str) -> Option<Self> {
        let upper = token.to_ascii_uppercase();
        match upper.as_str() {
            "TD" | "TB" => Some(Self::TopDown),
            "BT" => Some(Self::BottomTop),
            "LR" => Some(Self::LeftRight),
            "RL" => Some(Self::RightLeft),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
}

#[derive(Debug, Clone)]
pub struct NodeLink {
    pub url: String,
    pub title: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub start_label: Option<String>,
    pub end_label: Option<String>,
    pub directed: bool,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub arrow_start_kind: Option<EdgeArrowhead>,
    pub arrow_end_kind: Option<EdgeArrowhead>,
    pub start_decoration: Option<EdgeDecoration>,
    pub end_decoration: Option<EdgeDecoration>,
    pub style: EdgeStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDecoration {
    Circle,
    Cross,
    Diamond,
    DiamondFilled,
    // Crow's foot notation for ER diagrams
    CrowsFootOne,      // || exactly one
    CrowsFootZeroOne,  // o| zero or one
    CrowsFootMany,     // |{ one or many
    CrowsFootZeroMany, // o{ zero or many
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeArrowhead {
    OpenTriangle,
    ClassDependency,
}

#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: Option<String>,
    pub label: String,
    pub nodes: Vec<String>,
    pub direction: Option<Direction>,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub kind: DiagramKind,
    pub direction: Direction,
    pub nodes: BTreeMap<String, Node>,
    pub node_order: HashMap<String, usize>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    pub sequence_participants: Vec<String>,
    pub sequence_frames: Vec<SequenceFrame>,
    pub sequence_notes: Vec<SequenceNote>,
    pub sequence_activations: Vec<SequenceActivation>,
    pub sequence_autonumber: Option<usize>,
    pub sequence_boxes: Vec<SequenceBox>,
    pub state_notes: Vec<StateNote>,
    pub pie_slices: Vec<PieSlice>,
    pub pie_title: Option<String>,
    pub pie_show_data: bool,
    pub quadrant: QuadrantData,
    pub gantt_tasks: Vec<GanttTask>,
    pub gantt_title: Option<String>,
    pub gantt_sections: Vec<String>,
    pub gitgraph: GitGraphData,
    pub class_defs: HashMap<String, NodeStyle>,
    pub node_classes: HashMap<String, Vec<String>>,
    pub node_styles: HashMap<String, NodeStyle>,
    pub subgraph_styles: HashMap<String, NodeStyle>,
    pub subgraph_classes: HashMap<String, Vec<String>>,
    pub node_links: HashMap<String, NodeLink>,
    pub edge_styles: HashMap<usize, EdgeStyleOverride>,
    pub edge_style_default: Option<EdgeStyleOverride>,
    pub c4: C4Data,
    pub mindmap: MindmapData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    Rectangle,
    ForkJoin,
    RoundRect,
    Stadium,
    Subroutine,
    Cylinder,
    ActorBox,
    Circle,
    DoubleCircle,
    Diamond,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    Asymmetric,
    MindmapDefault,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindmapNodeType {
    Default,
    RoundedRect,
    Rect,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub id: String,
    pub label: String,
    pub level: usize,
    pub section: Option<usize>,
    pub node_type: MindmapNodeType,
    pub icon: Option<String>,
    pub class: Option<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MindmapData {
    pub nodes: Vec<MindmapNode>,
    pub root_id: Option<String>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            kind: DiagramKind::Flowchart,
            direction: Direction::TopDown,
            nodes: BTreeMap::new(),
            node_order: HashMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            sequence_participants: Vec::new(),
            sequence_frames: Vec::new(),
            sequence_notes: Vec::new(),
            sequence_activations: Vec::new(),
            sequence_autonumber: None,
            sequence_boxes: Vec::new(),
            state_notes: Vec::new(),
            pie_slices: Vec::new(),
            pie_title: None,
            pie_show_data: false,
            quadrant: QuadrantData::default(),
            gantt_tasks: Vec::new(),
            gantt_title: None,
            gantt_sections: Vec::new(),
            gitgraph: GitGraphData::default(),
            class_defs: HashMap::new(),
            node_classes: HashMap::new(),
            node_styles: HashMap::new(),
            subgraph_styles: HashMap::new(),
            subgraph_classes: HashMap::new(),
            node_links: HashMap::new(),
            edge_styles: HashMap::new(),
            edge_style_default: None,
            c4: C4Data::default(),
            mindmap: MindmapData::default(),
        }
    }

    pub fn ensure_node(&mut self, id: &str, label: Option<String>, shape: Option<NodeShape>) {
        let is_new = !self.nodes.contains_key(id);
        let entry = self.nodes.entry(id.to_string()).or_insert(Node {
            id: id.to_string(),
            label: id.to_string(),
            shape: NodeShape::Rectangle,
        });
        if is_new {
            let order = self.node_order.len();
            self.node_order.insert(id.to_string(), order);
        }
        if let Some(label) = label {
            entry.label = label;
        }
        if let Some(shape) = shape {
            entry.shape = shape;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub text_color: Option<String>,
    pub stroke_width: Option<f32>,
    pub stroke_dasharray: Option<String>,
    pub line_color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EdgeStyleOverride {
    pub stroke: Option<String>,
    pub stroke_width: Option<f32>,
    pub dasharray: Option<String>,
    pub label_color: Option<String>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
