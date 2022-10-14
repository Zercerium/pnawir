use std::{
    collections::{HashMap, HashSet},
    vec,
};

use crate::modular_net::{ModularPetrinet, ModuleId, PetrinetModul, TransitionId};

use super::graph::{
    Graph, GraphEdge, GraphMarking, Id, Marking, MarkingId, Segment, SyncEdge, SyncMarking,
};

pub fn build_sync_reachability_graph(net: &ModularPetrinet) -> Graph {
    // dbg!(&net);
    let net_count = net.modules.len();

    // create empty graph
    let mut graph = Graph {
        sync_graph: vec![],
        segment_storage: vec![(vec![], 0); net_count],
    };

    // get initial firable_list
    let mut firable_list = vec![vec![]; net_count];
    for id in 0..net_count {
        let firable = initial_firable(&net.modules[id], &net.markings[id]);
        firable_list[id] = firable;
    }

    // explore all initial segments
    let mut to_explore = vec![];
    let mut enabled_e_t = vec![vec![]; net_count];
    for id in 0..net_count {
        let seg_id = graph.segment_storage[id].0.len();
        let (new_segment, e_t, max_p_id) = explore_segment(
            &net.modules[id],
            &vec![net.markings[id].clone()],
            &vec![firable_list[id].clone()],
            seg_id as Id,
            net.intern_transition_start,
            graph.segment_storage[id].1,
        );

        graph.segment_storage[id].1 = max_p_id;
        graph.segment_storage[id].0.push((new_segment, vec![]));
        enabled_e_t[id] = e_t;
    }

    // seed to explore with first marking
    graph.sync_graph.push(SyncMarking {
        segment_ids: vec![0; net_count],
        edges: vec![],
    });
    to_explore.push((0, enabled_e_t));

    // sync_id, enabled_e_t
    while let Some(now_exploring) = to_explore.pop() {
        // Module[*(t_id, ma_id, firable*[t_id])]
        let enabled_t_info = now_exploring.1;
        let c_sync_marking = &graph.sync_graph[now_exploring.0].clone();
        // find all enabled extern transitions
        // all enabled found transitions, indexes enabled_e_t transitions for every module
        let indexes = find_all_e_t(&enabled_t_info, &net.extern_t_overview);
        let mut edges = vec![];
        // for every global enabled transition
        for (e_t_id, touched_modules) in &indexes {
            let mut new_sync_marking = SyncMarking {
                segment_ids: vec![0; net_count],
                edges: vec![],
            };

            // for every module
            let mut e_t_o2 = vec![];
            for m_id in 0..net_count {
                // in the modules where this transitions exist
                if touched_modules.contains(&(m_id as u16)) {
                    let mut seg_e_e = (*e_t_id, vec![]);
                    // fire transition
                    let mut pre_fire_marking = vec![];
                    let mut start_markings = vec![];
                    let mut start_firable = vec![];
                    let t_info = &enabled_t_info[m_id];
                    for t in t_info {
                        // only for real real extern_t // all t saved only fire current
                        if t.0 == *e_t_id {
                            let c_segment = &graph.segment_storage[m_id].0
                                [c_sync_marking.segment_ids[m_id] as usize]
                                .0;

                            let marking = &c_segment.markings
                                [(t.1 - c_segment.marking_offset) as usize]
                                .marking;
                            assert_eq!(
                                c_segment.markings[(t.1 - c_segment.marking_offset) as usize].id,
                                t.1
                            );
                            pre_fire_marking.push(
                                &c_segment.markings[(t.1 - c_segment.marking_offset) as usize].id,
                            );
                            let x = fire(&net.modules[m_id], marking, &t.2, *e_t_id);
                            start_markings.push(x.0);
                            start_firable.push(x.1);
                        }
                    }

                    // build new segment
                    let x = explore_segment(
                        &net.modules[m_id],
                        &start_markings,
                        &start_firable,
                        graph.segment_storage[m_id].0.len() as u32,
                        net.intern_transition_start,
                        graph.segment_storage[m_id].1, // place_start_id
                    );

                    for (i, _) in start_markings.iter().enumerate() {
                        seg_e_e.1.push((
                            *pre_fire_marking[i],
                            i as u32 + graph.segment_storage[m_id].1,
                        ))
                    }

                    // compare with current segments
                    let seg_id;
                    if let Some(x) = graph.contains_segment(&x.0, m_id as u16) {
                        seg_id = x;
                    } else {
                        seg_id = graph.segment_storage[m_id].0.len() as u32;
                        graph.segment_storage[m_id].0.push((x.0, vec![]));
                        graph.segment_storage[m_id].1 = x.2;
                        // store segments
                    }
                    new_sync_marking.segment_ids[m_id] = seg_id;

                    // link correct marking to marking

                    e_t_o2.push(x.1);
                    // save segment edges for e_t

                    graph.segment_storage[m_id].0[c_sync_marking.segment_ids[m_id] as usize]
                        .1
                        .push(seg_e_e.clone());
                }
                // in the modules where this transitions did not exist
                else {
                    // just add the current seg_id
                    // nothing changes
                    e_t_o2.push(enabled_t_info[m_id].clone());
                    new_sync_marking.segment_ids[m_id] = c_sync_marking.segment_ids[m_id];
                }
            }

            // same sync_node
            if new_sync_marking == *c_sync_marking {
                // links segments (edges)
                edges.push(SyncEdge {
                    transition_id: *e_t_id,
                    sync_marking_id: now_exploring.0 as u32,
                });
                // recreate seg -> seg edges

                for m_id in 0..net_count {
                    let old = graph.segment_storage[m_id].0
                        [c_sync_marking.segment_ids[m_id] as usize]
                        .1
                        .pop()
                        .unwrap();

                    graph.segment_storage[m_id].0[c_sync_marking.segment_ids[m_id] as usize]
                        .1
                        .push((*e_t_id, vec![]));

                    // TODO search for marking_id
                }
            } else {
                // links segments (edges)
                edges.push(SyncEdge {
                    transition_id: *e_t_id,
                    sync_marking_id: (graph.sync_graph.len() as u32),
                });
                to_explore.push((graph.sync_graph.len(), e_t_o2));
                let sync_g = &mut graph.sync_graph;
                sync_g.push(new_sync_marking);
            }
        }
        let sync_g = &mut graph.sync_graph;
        sync_g[now_exploring.0].edges = edges;

        // graph.print(net);
    }

    dbg!(&graph);
    graph
}

// need to know where extern transitions
/// return vector index where all saved
fn find_all_e_t(
    ex_t: &Vec<Vec<(TransitionId, MarkingId, Vec<TransitionId>)>>,
    extern_t_overview: &Vec<Vec<ModuleId>>,
) -> Vec<(TransitionId, Vec<u16>)> {
    let mut found_t = HashMap::<u32, HashSet<u16>>::new();

    // look for activated transitions
    // collect all
    for (m_id, m) in ex_t.iter().enumerate() {
        for (t_id, _, _) in m {
            found_t
                .entry(*t_id)
                .and_modify(|e| {
                    e.insert(m_id as u16);
                })
                .or_insert(HashSet::from([m_id as u16]));
        }
    }

    let mut result = vec![];
    // get all actiavted transitions
    // calculate if activated
    for t in found_t {
        // all activated
        if t.1.len() == extern_t_overview[t.0 as usize].len() {
            // let mut found_ids = vec![];
            // for m_id in t.1 {
            //     for (pos, t_ex_t) in ex_t[m_id as usize].iter().enumerate() {
            //         if t_ex_t.0 == t.0 {
            //             found_ids.push(pos);
            //         }
            //     }
            // }
            let mut res = t.1.into_iter().collect::<Vec<_>>();
            res.sort();
            result.push((t.0, res));
        }
    }

    result
}
// urspruenglich mal anders gedacht
// referenziert alle ids im vektor für den schnelleren zugriff

fn initial_firable(module: &PetrinetModul, marking: &Marking) -> Vec<TransitionId> {
    // dbg!(&module.name);
    // dbg!(&module.marking);
    let mut firable = vec![];
    't: for t in module.transitions.iter() {
        if t.output_places.len() + t.input_places.len() == 0 {
            continue;
        }
        for input in t.input_places.iter() {
            let count = marking.count(input.0);
            if input.1 > count {
                continue 't;
            }
        }
        firable.push(t.id);
    }
    // dbg!(&firable);
    firable
}

fn explore_segment(
    module: &PetrinetModul,
    marking: &Vec<Marking>,
    firable: &Vec<Vec<TransitionId>>,
    seg_id: Id,
    intern_start: TransitionId,
    start_marking_id: MarkingId,
) -> (
    Segment,
    Vec<(TransitionId, MarkingId, Vec<TransitionId>)>,
    MarkingId,
) {
    assert_eq!(marking.len(), firable.len());
    // collect all extern firable
    // dont fire extern t in local segment
    // t_id, marking_id, Firable
    let mut extern_firable = vec![];
    // (marking_id, Firable)
    let mut to_explore = vec![];
    for (i, m) in firable.iter().enumerate() {
        for &t in m {
            // println!("{},{}", t, intern_start);
            if t < intern_start {
                // println!("{}",t);
                extern_firable.push((t, i as u32 + start_marking_id, m.clone()));
            }
        }
    }

    let mut segment = Segment {
        id: seg_id,
        marking_offset: start_marking_id,
        markings: vec![],
    };

    for m in marking {
        let id = start_marking_id + segment.markings.len() as u32;
        let graph_marking = GraphMarking {
            id,
            marking: m.clone(),
            edges: vec![],
        };
        to_explore.push((id, firable[segment.markings.len()].clone()));
        segment.markings.push(graph_marking);
    }

    while let Some(now_exploring) = to_explore.pop() {
        let m_id = now_exploring.0 - segment.marking_offset;
        let mut edges = vec![];
        let marking = &segment.markings[m_id as usize].marking.clone();
        for &t_id in &now_exploring.1 {
            if t_id < intern_start {
                continue;
            }
            let (new_marking, mut new_firable) = fire(module, marking, &now_exploring.1, t_id);
            // dbg!(t_id);
            // dbg!(&new_marking);

            // check if marking exists
            let mark_id;
            if let Some(x) = segment.search_equal_marking(&new_marking) {
                mark_id = x;
            } else {
                mark_id = segment.marking_offset + segment.markings.len() as u32;
                let new_graph_marking = GraphMarking {
                    id: mark_id,
                    marking: new_marking,
                    edges: vec![],
                };

                // extra case extern firable
                new_firable.sort_unstable();
                let part_point = new_firable.partition_point(|&x| x < intern_start);
                let (e_t, _) = new_firable.split_at(part_point);

                for &t in e_t {
                    extern_firable.push((t, mark_id, new_firable.clone()));
                }

                to_explore.push((mark_id, new_firable.clone()));
                segment.markings.push(new_graph_marking);
            }

            edges.push(GraphEdge::new(t_id, mark_id));
        }
        let graph_marking = &mut segment.markings[m_id as usize];
        graph_marking.edges = edges;
    }

    let start_marking_id = start_marking_id + segment.markings.len() as u32;
    // segment.print(module);
    (segment, extern_firable, start_marking_id)
}

fn fire(
    module: &PetrinetModul,
    marking: &Marking,
    firable: &Vec<TransitionId>,
    id: TransitionId,
) -> (Marking, Vec<TransitionId>) {
    // dbg!(&marking);
    let mut marking = marking.clone();
    let mut firable = firable.clone();
    let transition = &module.transitions[id as usize];
    // dbg!(&transition);

    // update input places
    for place in transition.input_places.iter() {
        // update marking
        // dbg!(place);
        let new_amount = marking.update(place.0, place.1, false);

        // remove from firable list (can only deactivate)
        for t in module.places[place.0 as usize].output_transitions.iter() {
            // skip if not in firable list
            if let Some(x) = firable.iter().position(|&x| x == t.0) {
                // remove if count is to low
                if t.1 > new_amount {
                    firable.swap_remove(x);
                    continue;
                }
            }
        }
    }

    // dbg!(&marking);

    // update output places
    for place in transition.output_places.iter() {
        // update marking
        marking.update(place.0, place.1, true);

        // add to firable list (can only activate)
        't: for t in module.places[place.0 as usize].output_transitions.iter() {
            // skip if in firable list
            if firable.contains(&t.0) {
                continue;
            }

            // else check all input_places
            for p in module.transitions[t.0 as usize].input_places.iter() {
                // short circuit, if first place has to few marks
                if p.1 > marking.count(p.0) {
                    continue 't;
                }
            }

            firable.push(t.0);
        }
    }
    // dbg!(&marking);
    (marking, firable)
}

// damit die Datenstruktur nicht verändert werden muss und
// immer referenziert werden kann!
