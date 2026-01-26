use graphlib_rust::{Graph, GraphOption};
use crate::{GraphConfig, GraphEdge, GraphEdgePoint, GraphNode};
use crate::layout::add_border_segments::add_border_segments;
use crate::layout::order::order;
use crate::layout::parent_dummy_chains::parent_dummy_chains;
use crate::layout::rank::rank;
use crate::layout::util::{as_non_compound_graph, intersect_rect, normalize_ranks, Rect, remove_empty_ranks, transfer_node_edge_labels};

pub mod acyclic;
pub mod nesting_graph;
pub mod util;
pub mod rank;
pub mod normalize;
pub mod parent_dummy_chains;
pub mod add_border_segments;
pub mod order;
pub mod coordinate_system;
pub mod position;

const DEFAULT_RANK_SEP: f32 = 50.0;

pub fn layout(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let mut layout_graph = build_layout_graph(g);
  run_layout(&mut layout_graph);
  update_input_graph(g, &layout_graph);
}

pub fn update_input_graph(
  input_graph: &mut Graph<GraphConfig, GraphNode, GraphEdge>,
  layout_graph: &Graph<GraphConfig, GraphNode, GraphEdge>,
) {
  for v in input_graph.nodes() {
    let input_label_ = input_graph.node_mut(&v);
    let layout_label = layout_graph.node(&v).unwrap();

    if let Some(input_label) = input_label_ {
      input_label.x = layout_label.x;
      input_label.y = layout_label.y;

      if layout_graph.children(&v).len() > 0 {
        input_label.width = layout_label.width;
        input_label.height = layout_label.height;
      }
    }
  }

  for e in input_graph.edges() {
    let input_label = input_graph.edge_mut_with_obj(&e).unwrap();
    let layout_label = layout_graph.edge_with_obj(&e).unwrap();

    input_label.points = layout_label.points.clone();
    input_label.x = layout_label.x;
    input_label.y = layout_label.y;
  }

  input_graph.graph_mut().width = layout_graph.graph().width;
  input_graph.graph_mut().height = layout_graph.graph().height;
}

pub fn set_graph_label_default_values(graph_label: &mut GraphConfig) {
  if graph_label.ranksep.is_none() {
    graph_label.ranksep = Some(50.0);
  }

  if graph_label.edgesep.is_none() {
    graph_label.edgesep = Some(20.0);
  }

  if graph_label.nodesep.is_none() {
    graph_label.nodesep = Some(50.0);
  }

  if graph_label.rankdir.is_none() {
    graph_label.rankdir = Some("tb".to_string());
  }

  if graph_label.marginx.is_none() {
    graph_label.marginx = Some(0.0);
  }

  if graph_label.marginy.is_none() {
    graph_label.marginy = Some(0.0);
  }
}

pub fn set_edge_label_default_values(edge_label: &mut GraphEdge) {
  if edge_label.minlen.is_none() {
    edge_label.minlen = Some(1.0);
  }

  if edge_label.weight.is_none() {
    edge_label.weight = Some(1.0);
  }

  if edge_label.width.is_none() {
    edge_label.width = Some(0.0);
  }

  if edge_label.height.is_none() {
    edge_label.height = Some(0.0);
  }

  if edge_label.labeloffset.is_none() {
    edge_label.labeloffset = Some(10.0);
  }

  if edge_label.labelpos.is_none() {
    edge_label.labelpos = Some("r".to_string());
  }
}

/*
 * Constructs a new graph from the input graph, which can be used for layout.
 * This process copies only whitelisted attributes from the input graph to the
 * layout graph. Thus this function serves as a good place to determine what
 * attributes can influence layout.
 */
pub fn build_layout_graph(input_graph: &Graph<GraphConfig, GraphNode, GraphEdge>) -> Graph<GraphConfig, GraphNode, GraphEdge> {
  let mut g: Graph<GraphConfig, GraphNode, GraphEdge> = Graph::new(Some(GraphOption {
    directed: Some(true),
    multigraph: Some(true),
    compound: Some(true)
  }));

  let mut graph_label: GraphConfig = input_graph.graph().clone();
  set_graph_label_default_values(&mut graph_label);
  g.set_graph(graph_label);

  for node_id in input_graph.nodes().iter() {
    let _node = input_graph.node(node_id);
    if _node.is_none() {
      continue;
    }
    g.set_node(node_id.clone(), _node.cloned());
    let _ = g.set_parent(node_id, input_graph.parent(node_id).cloned());
  }

  for edge_obj in input_graph.edges() {
    let _edge = input_graph.edge_with_obj(&edge_obj);
    if _edge.is_none() {
      continue;
    }
    let mut edge_label = _edge.cloned().unwrap();
    set_edge_label_default_values(&mut edge_label);
    let _ = g.set_edge_with_obj(&edge_obj, Some(edge_label));
  }

  return g;
}

/*
 * This idea comes from the Gansner paper: to account for edge labels in our
 * layout we split each rank in half by doubling minlen and halving ranksep.
 * Then we can place labels at these mid-points between nodes.
 *
 * We also add some minimal padding to the width to push the label for the edge
 * away from the edge itself a bit.
 */
pub fn make_space_for_edge_labels(graph: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let graph_config = graph.graph_mut();
  graph_config.ranksep = Some(graph_config.ranksep.unwrap_or(DEFAULT_RANK_SEP) / 2.0);

  // moving in nested block due to borrow checker
  {
    let graph_config = graph.graph().clone();
    let edge_objs = graph.edges();
    for edge_obj in edge_objs.into_iter() {
      let _edge = graph.edge_mut_with_obj(&edge_obj);
      if _edge.is_none() {
        continue;
      }
      let edge = _edge.unwrap();

      let minlen = edge.minlen.unwrap_or(1.0);
      let labelpos = edge.labelpos.clone().unwrap_or("".to_string());
      let labeloffset = edge.labeloffset.unwrap_or(10.0);
      let rankdir = graph_config.rankdir.clone().unwrap_or("".to_string());

      edge.minlen = Some(minlen * 2.0);
      if labelpos != "c" {
        if rankdir == "tb" || rankdir == "bt" {
          edge.width = Some(edge.width.unwrap_or(0.0) + labeloffset);
        } else {
          edge.height = Some(edge.height.unwrap_or(0.0) + labeloffset);
        }
      }
    }
  }
}

/*
 * Creates temporary dummy nodes that capture the rank in which each edge's
 * label is going to, if it has one of non-zero width and height. We do this
 * so that we can safely remove empty ranks while preserving balance for the
 * label's position.
 */
pub fn inject_edge_label_proxies(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let edges = g.edges();
  for e in edges.into_iter() {
    let edge_ = g.edge_with_obj(&e);
    if let Some(edge) = edge_ {
      if edge.width.clone().unwrap_or(0.0) > 0.0 && edge.height.clone().unwrap_or(0.0) > 0.0 {
        let v = g.node(&e.v);
        let w = g.node(&e.w);
        let v_rank = v.cloned().unwrap_or(GraphNode::default()).rank.unwrap_or(0);
        let w_rank = w.cloned().unwrap_or(GraphNode::default()).rank.unwrap_or(0);
        let mut label = GraphNode::default();
        label.rank = Some((w_rank - v_rank) / 2 + v_rank);
        label.e = Some(e.clone());
        util::add_dummy_node(g, "edge-proxy".to_string(), label, "_ep".to_string());
      }
    }
  }
}

pub fn assign_rank_min_max(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let mut max_rank = 0;
  let vs = g.nodes();
  for v in vs.iter() {
    let node_ = g.node(v);
    if node_.is_none() {
      continue;
    }
    let node = node_.unwrap();
    if node.border_top.is_some() {
      let border_top = node.border_top.clone().unwrap();
      let border_bottom = node.border_bottom.clone().unwrap();
      let _min_rank = g.node(&border_top).cloned().unwrap().rank.unwrap_or(0);
      let _max_rank = g.node(&border_bottom).cloned().unwrap().rank.unwrap_or(0);

      let _node = g.node_mut(v).unwrap();
      _node.min_rank = Some(_min_rank);
      _node.max_rank = Some(_max_rank.clone());
      max_rank = std::cmp::max(max_rank, _max_rank);
    }
  }
}

enum GraphElement<'a> {
  Node(&'a GraphNode),
  Edge(&'a GraphEdge),
}

impl<'a> GraphElement<'a> {
  fn x(&self) -> f32 {
    match self {
      GraphElement::Node(node) => node.x,
      GraphElement::Edge(edge) => edge.x,
    }
  }

  fn y(&self) -> f32 {
    match self {
      GraphElement::Node(node) => node.y,
      GraphElement::Edge(edge) => edge.y,
    }
  }

  fn width(&self) -> f32 {
    match self {
      GraphElement::Node(node) => node.width,
      GraphElement::Edge(edge) => edge.width.unwrap_or(0.0),
    }
  }

  fn height(&self) -> f32 {
    match self {
      GraphElement::Node(node) => node.height,
      GraphElement::Edge(edge) => edge.height.unwrap_or(0.0),
    }
  }
}

pub fn translate_graph(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let mut min_x = f64::INFINITY as f32;
  let mut max_x: f32 = 0.0;
  let mut min_y = f64::INFINITY as f32;
  let mut max_y: f32 = 0.0;
  let mut graph_label = g.graph().clone();
  let margin_x = graph_label.marginx.unwrap_or(0.0);
  let margin_y = graph_label.marginy.unwrap_or(0.0);

  fn get_extremes(attrs: &GraphElement, min_x: &mut f32, max_x: &mut f32, min_y: &mut f32, max_y: &mut f32) {
    let x = attrs.x();
    let y = attrs.y();
    let w = attrs.width();
    let h = attrs.height();
    *min_x = min_x.min(x - w / 2.0);
    *max_x = max_x.max(x + w / 2.0);
    *min_y = min_y.min(y - h / 2.0);
    *max_y = max_y.max(y + h / 2.0);
  }

  for v in g.nodes() {
    get_extremes(&GraphElement::Node(g.node(&v).unwrap()), &mut min_x, &mut max_x, &mut min_y, &mut max_y);
  }

  for e in g.edges() {
    let edge = g.edge_with_obj(&e).unwrap();
    if edge.x != 0.0 {
      get_extremes(&GraphElement::Edge(edge), &mut min_x, &mut max_x, &mut min_y, &mut max_y);
    }
  }

  min_x -= margin_x;
  min_y -= margin_y;

  for v in g.nodes() {
    let mut node = g.node_mut(&v).unwrap();
    node.x -= min_x;
    node.y -= min_y;
  }

  for e in g.edges() {
    let edge = g.edge_mut_with_obj(&e).unwrap();
    if edge.points.is_some() {
      for p in edge.points.as_mut().unwrap() {
        p.x -= min_x;
        p.y -= min_y;
      }
    }
    if edge.x != 0.0 {
      edge.x = min_x;
    }

    if edge.y != 0.0 {
      edge.y = min_y;
    }
  }

  graph_label.width = max_x - min_x + margin_x;
  graph_label.height = max_y - min_y + margin_y;

  g.set_graph(graph_label);
}

pub fn assign_node_intersects(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  for e in g.edges() {
    let mut edge = g.edge_mut_with_obj(&e).cloned().unwrap();
    let node_v = g.node(&e.v).cloned().unwrap();
    let node_w = g.node(&e.w).cloned().unwrap();
    let (p1, p2) = if edge.points.is_none() {
      edge.points = Some(vec![]);
      (GraphEdgePoint {
        x: node_w.x,
        y: node_w.y
      }, GraphEdgePoint {
        x: node_v.x,
        y: node_v.y
      })
    } else {
      let points = edge.points.clone().unwrap();
      let r1 = GraphEdgePoint {
        x: points[0].x,
        y: points[0].y
      };

      let r2 = GraphEdgePoint {
        x: points[points.len() - 1].x,
        y: points[points.len() - 1].y
      };

      (r1, r2)
    };

    let points = edge.points.as_mut().unwrap();
    points.insert(0, intersect_rect(&Rect {
      x: node_v.x,
      y: node_v.y,
      width: node_v.width,
      height: node_v.height
    }, &p1));

    points.push(intersect_rect(&Rect {
      x: node_w.x,
      y: node_w.y,
      width: node_w.width,
      height: node_w.height
    }, &p2));

    let _ = g.set_edge_with_obj(&e, Some(edge));
  }
}

pub fn remove_edge_label_proxies(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let vs = g.nodes();
  for v in vs.iter() {
    let node = g.node(v).unwrap();
    if node.dummy.is_some() && node.dummy.clone().unwrap() == "edge-proxy" {
      let rank = node.rank.unwrap_or(0);
      let graph_edge_ = g.edge_mut_with_obj(&node.e.clone().unwrap());
      if let Some(graph_edge) = graph_edge_ {
        graph_edge.label_rank = Some(rank);
      }
      g.remove_node(v);
    }
  }
}

pub fn fixup_edge_label_coords(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  g.edges().iter().for_each(|e| {
    let edge = g.edge_mut_with_obj(&e.to_owned()).unwrap();
    if edge.x != 0.0 {
      let labelpos = edge.labelpos.clone().unwrap_or("".to_string());
      let labeloffset = edge.labeloffset.clone().unwrap_or(0.0);
      if labelpos == "l" || labelpos == "r" {
        edge.width = Some(edge.width.unwrap_or(0.0) - labeloffset);
      }

      if labelpos == "l" {
        edge.x -= edge.width.clone().unwrap_or(0.0) / 2.0 + labeloffset;
      } else if labelpos == "r" {
        edge.x += edge.width.clone().unwrap_or(0.0) / 2.0 + labeloffset;
      }
    }
  });
}

pub fn reverse_points_for_reversed_edges(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  for e in g.edges() {
    let edge = g.edge_mut_with_obj(&e.to_owned()).unwrap();
    if edge.reversed.clone().unwrap_or(false) {
      if edge.points.is_some() {
        let points = edge.points.as_mut().unwrap();
        points.reverse();
      }
    }
  }
}

pub fn remove_border_nodes(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  for v in g.nodes() {
    if g.children(&v).len() > 0 {
      let mut node = g.node(&v).cloned().unwrap();
      let (Some(border_top), Some(border_bottom), Some(border_left), Some(border_right)) = (
        node.border_top.clone(),
        node.border_bottom.clone(),
        node.border_left.clone(),
        node.border_right.clone(),
      ) else {
        continue;
      };
      let t = g.node(&border_top).cloned().unwrap();
      let b = g.node(&border_bottom).cloned().unwrap();
      // TODO: improve this after wasm implementation
      let mut l_keys: Vec<i32> = border_left.keys().cloned().collect();
      l_keys.sort();
      let mut r_keys: Vec<i32> = border_right.keys().cloned().collect();
      r_keys.sort();
      if l_keys.is_empty() || r_keys.is_empty() {
        continue;
      }
      let l = g.node(border_left.get(&l_keys[l_keys.len() - 1]).unwrap()).cloned().unwrap();
      let r = g.node(border_right.get(&r_keys[r_keys.len() - 1]).unwrap()).cloned().unwrap();

      node.width = (r.x - l.x).abs();
      node.height = (b.y - t.y).abs();
      node.x = l.x + node.width / 2.0;
      node.y = l.y + node.height / 2.0;

      g.set_node(v.clone(), Some(node));
    }
  }

  g.nodes().iter().for_each(|v| {
    let node = g.node(v).unwrap();
    if node.dummy.is_some() && node.dummy.clone().unwrap() == "border" {
      g.remove_node(v);
    }
  });
}

pub fn remove_self_edges(graph: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let edge_objs = graph.edges();
  for edge_obj in edge_objs.into_iter() {
    if edge_obj.v == edge_obj.w {
      let edge_label = graph.edge_with_obj(&edge_obj).cloned().unwrap();
      let node = graph.node_mut(&edge_obj.v).unwrap();
      node.self_edges.push((edge_obj.clone(), edge_label));
      graph.remove_edge_with_obj(&edge_obj);
    }
  }
}

pub fn insert_self_edges(graph: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  let layers = util::build_layer_matrix(graph);
  layers.iter().for_each(|layer| {
    let mut order_shift = 0;
    layer.iter().enumerate().for_each(|(i, v)| {
      let node = graph.node_mut(v).unwrap();
      node.order = Some(i + order_shift);
      let rank = node.rank.clone();

      let self_edges = node.self_edges.clone();
      self_edges.into_iter().for_each(|(edge, graph_edge)| {
        let mut _graph_node = GraphNode::default();
        _graph_node.width = graph_edge.width.clone().unwrap_or(0.0);
        _graph_node.height = graph_edge.height.clone().unwrap_or(0.0);
        _graph_node.rank = rank.clone();
        order_shift += 1;
        _graph_node.order = Some(i + order_shift);
        _graph_node.e = Some(edge.clone());
        _graph_node.label = Some(graph_edge.clone());
        util::add_dummy_node(graph, "selfedge".to_string(), _graph_node, "_se".to_string());
      });
    });
  })
}

pub fn position_self_edges(g: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  for v in g.nodes() {
    let node = g.node(&v).cloned().unwrap();
    if node.dummy.unwrap_or("".to_string()) == "selfedge" {
      let self_node = g.node(&node.e.as_ref().unwrap().v).unwrap();
      let x = self_node.x + self_node.width / 2.0;
      let y = self_node.y;
      let dx = node.x - x;
      let dy = self_node.height / 2.0;
      let mut graph_edge = node.label.clone().unwrap();
      graph_edge.points = Some(vec![
        GraphEdgePoint { x: x + 2.0 * dx / 3.0, y: y - dy },
        GraphEdgePoint { x: x + 2.0 * dx / 3.0, y: y - dy },
        GraphEdgePoint { x: x + 5.0 * dx / 6.0, y: y - dy },
        GraphEdgePoint { x: x +     dx        , y },
        GraphEdgePoint { x: x + 5.0 * dx / 6.0, y: y + dy },
        GraphEdgePoint { x: x + 2.0 * dx / 3.0, y: y + dy }
      ]);
      graph_edge.x = node.x;
      graph_edge.y = node.y;
      let _ = g.set_edge_with_obj(&node.e.unwrap(), Some(graph_edge));
      g.remove_node(&v);
    }
  }
}

pub fn run_layout(graph: &mut Graph<GraphConfig, GraphNode, GraphEdge>) {
  make_space_for_edge_labels(graph);
  remove_self_edges(graph);
  acyclic::run(graph);
  nesting_graph::run(graph);
  // calculating ranks
  let mut nc_graph: Graph<GraphConfig, GraphNode, GraphEdge> = as_non_compound_graph(graph);
  rank(&mut nc_graph);
  transfer_node_edge_labels(&nc_graph, graph);
  // done with calculating ranks
  inject_edge_label_proxies(graph);
  remove_empty_ranks(graph);
  nesting_graph::cleanup(graph);
  normalize_ranks(graph);
  assign_rank_min_max(graph);
  remove_edge_label_proxies(graph);
  normalize::run(graph);
  parent_dummy_chains(graph);
  add_border_segments(graph);
  order(graph);
  insert_self_edges(graph);
  coordinate_system::adjust(graph);
  position::position(graph);
  position_self_edges(graph);
  remove_border_nodes(graph);
  normalize::undo(graph);
  fixup_edge_label_coords(graph);
  coordinate_system::undo(graph);
  translate_graph(graph);
  assign_node_intersects(graph);
  reverse_points_for_reversed_edges(graph);
  acyclic::undo(graph);
}
