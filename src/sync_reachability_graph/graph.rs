use std::cmp::Ordering;

use crate::modular_net::{ModularPetrinet, ModuleId, PetrinetModul, PlaceId, TransitionId, Weight};

pub type Id = u32;
type Count = u32;
pub type MarkingId = Id;
pub type SegmentId = Id;

#[derive(Debug)]
pub struct Graph {
    pub sync_graph: Vec<SyncMarking>,
    pub segment_storage: Vec<(
        Vec<(Segment, Vec<(TransitionId, Vec<(MarkingId, MarkingId)>)>)>,
        MarkingId, // current marking count overall
    )>,
}

impl Graph {
    pub fn contains_sync_node(&self, sm_b: SyncMarking) -> Option<usize> {
        self.sync_graph.iter().position(|sm_a| sm_a == &sm_b)
    }

    pub fn contains_segment(&self, segment: &Segment, m_id: ModuleId) -> Option<SegmentId> {
        let segs = &self.segment_storage[m_id as usize].0;
        let segment_sort = segment.sort_marking();
        for (idx, (seg, _)) in segs.iter().enumerate() {
            if seg.markings.len() != segment.markings.len() {
                continue;
            }
            let seg_sort = seg.sort_marking();
            if segment_sort == seg_sort {
                return Some(idx as u32);
            }
        }

        None
    }

    pub fn print(&self, net: &ModularPetrinet) {
        for (id, node) in self.sync_graph.iter().enumerate() {
            println!("SyncId: {} | Segments: {:?}", id, node.segment_ids);

            for edge in &node.edges {
                let mut t_name = "";
                let mut ni = 0;
                while t_name == "" {
                    t_name = &net.modules[ni].transitions[edge.transition_id as usize].name;
                    ni += 1;
                }
                println!("  {} -> {}", t_name, edge.sync_marking_id);
            }
            println!();
        }

        println!();
        println!("Segments");
        for module in 0..self.segment_storage.len() {
            println!("  Module: {}", module);
            for segment in &self.segment_storage[module].0 {
                segment.0.print(&net.modules[module], 4);

                let e_ts = &segment.1;
                for e_t in e_ts {
                    let mut t_name = "";
                    let mut ni = 0;
                    while t_name == "" {
                        t_name = &net.modules[ni].transitions[e_t.0 as usize].name;
                        ni += 1;
                    }
                    println!("    {}", t_name);
                    for t in &e_t.1 {
                        println!("      {} -> {}", t.0, t.1);
                    }
                    println!();
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncMarking {
    pub segment_ids: Vec<Id>,
    pub edges: Vec<SyncEdge>,
}

impl PartialEq for SyncMarking {
    fn eq(&self, other: &Self) -> bool {
        assert_eq!(self.segment_ids.len(), other.segment_ids.len());
        for (a, b) in self.segment_ids.iter().zip(other.segment_ids.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SyncEdge {
    pub transition_id: Id,
    pub sync_marking_id: Id,
}

impl SyncEdge {
    pub fn new(t_id: Id, sm_id: Id) -> Self {
        SyncEdge {
            transition_id: t_id,
            sync_marking_id: sm_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub id: Id,
    pub marking_offset: Id,
    pub markings: Vec<GraphMarking>,
}

impl Segment {
    pub fn search_equal_marking(&self, marking: &Marking) -> Option<MarkingId> {
        for m in &self.markings {
            if m.marking.eq(marking) {
                return Some(m.id as u32);
            }
        }
        None
    }

    fn sort_marking(&self) -> Vec<GraphMarking> {
        let mut result = self.markings.clone();
        result.sort_unstable_by(|a, b| a.marking.cmp(&b.marking));
        result
    }

    pub fn print(&self, module: &PetrinetModul, pre_spacing: usize) {
        let space = " ".repeat(pre_spacing);
        println!("{}SegmentId: {}", space, self.id);
        for m in &self.markings {
            println!("{}  MarkingId: {}", space, m.id);
            // println!("PlaceCount");
            print!("{}    ", space);
            for p in &m.marking.place_counts {
                let p_real = &module.places[p.place_id as usize];
                print!("{}({}), ", p_real.name, p.count);
            }
            println!();
            for edge in &m.edges {
                let t_real = &module.transitions[edge.transition_id as usize];
                println!(
                    "{}      {} -> {}",
                    space, t_real.name, edge.graph_marking_id
                );
            }
            println!();
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphMarking {
    pub id: Id,
    pub marking: Marking,
    pub edges: Vec<GraphEdge>,
}

impl PartialEq for GraphMarking {
    fn eq(&self, other: &Self) -> bool {
        self.marking == other.marking
    }
}

#[derive(Debug, Clone)]
pub struct Marking {
    pub place_counts: Vec<PlaceCount>,
}

impl Marking {
    pub fn count(&self, id: PlaceId) -> Count {
        let found = self.place_counts.binary_search_by(|p| p.place_id.cmp(&id));
        match found {
            Ok(x) => self.place_counts[x].count,
            Err(_) => 0,
        }
    }

    pub fn sort(&mut self) {
        self.place_counts
            .sort_by(|a, b| a.place_id.cmp(&b.place_id));
    }

    pub fn update(&mut self, id: PlaceId, amount: u32, add: bool) -> Weight {
        let found = self.place_counts.binary_search_by(|p| p.place_id.cmp(&id));
        match found {
            Ok(x) => {
                if add {
                    self.place_counts[x].count += amount;
                } else {
                    self.place_counts[x].count -= amount;
                    if self.place_counts[x].count == 0 {
                        self.place_counts.remove(x);
                        return 0;
                    }
                }
                return self.place_counts[x].count;
            }
            Err(x) => {
                self.place_counts.insert(x, PlaceCount::new(id, amount));
                return amount;
            }
        }
    }
}

impl PartialEq for Marking {
    fn eq(&self, other: &Self) -> bool {
        // places has to be sorted
        // at first a hash could be compared
        // splitting and multithreaded comparison is possible
        if self.place_counts.len() != other.place_counts.len() {
            return false;
        }
        let iter = self.place_counts.iter().zip(other.place_counts.iter());
        for place_count in iter {
            if place_count.0.place_id != place_count.1.place_id {
                return false;
            }
            if place_count.0.count != place_count.1.count {
                return false;
            }
        }
        true
    }
}

impl Eq for Marking {}

impl PartialOrd for Marking {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Marking {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.place_counts.len() < other.place_counts.len() {
            return Ordering::Less;
        }
        if self.place_counts.len() > other.place_counts.len() {
            return Ordering::Greater;
        }
        for (a, b) in self.place_counts.iter().zip(other.place_counts.iter()) {
            if a.place_id < b.place_id {
                return Ordering::Less;
            }
            if a.place_id > b.place_id {
                return Ordering::Greater;
            }
            if a.count < b.count {
                return Ordering::Less;
            }
            if a.count > b.count {
                return Ordering::Greater;
            }
        }
        Ordering::Equal
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlaceCount {
    pub place_id: PlaceId,
    pub count: u32,
}

impl PlaceCount {
    pub fn new(place_id: Id, count: u32) -> Self {
        PlaceCount { place_id, count }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct GraphEdge {
    pub transition_id: Id,
    pub graph_marking_id: Id,
}

impl GraphEdge {
    pub fn new(transition_id: Id, graph_marking_id: Id) -> Self {
        GraphEdge {
            transition_id,
            graph_marking_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sync_reachability_graph::graph::{Marking, PlaceCount};

    #[test]
    fn test_marking_update() {
        let mut marking = Marking {
            place_counts: vec![PlaceCount {
                place_id: 2,
                count: 1,
            }],
        };
        marking.update(2, 1, false);
        assert_eq!(
            marking,
            Marking {
                place_counts: vec![]
            }
        );
    }
}
