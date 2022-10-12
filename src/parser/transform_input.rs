use std::collections::{HashMap, HashSet};

use crate::{
    modular_net::{ModularPetrinet, PetrinetModul, Place, Transition, Weight},
    sync_reachability_graph::graph::{Marking, PlaceCount},
};

use super::parse_input::{RawParserInput, RawParserModule, RawParserPlace, RawParserTransition};

pub fn transform(input: RawParserInput) -> ModularPetrinet {
    // build place -> module Map
    let mut place_to_module = HashMap::new();
    for (i, module) in input.modules.iter().enumerate() {
        for place in module.places.iter() {
            place_to_module.insert(place.name.clone(), i);
        }
    }

    // find all interface transitions split transitions to modules
    let mut interface_transitions = Vec::new();
    let mut module_transitions = vec![vec![]; input.modules.len()];
    for transition in input.transitions.iter() {
        let belongs_to = transition_belongs_module(transition, &place_to_module);
        if belongs_to.len() > 1 {
            // extern transition
            let id = interface_transitions.len() as u32;
            interface_transitions.push((transition.name.clone(), id));
        }
        // add transition to specific modules for further processing
        for module_id in belongs_to {
            module_transitions[module_id as usize].push(transition);
        }
    }

    let intern_transition_start = interface_transitions.len() as u32;

    let mut modules = Vec::with_capacity(input.modules.len());
    let mut markings = Vec::with_capacity(input.modules.len());
    for (id, module) in input.modules.iter().enumerate() {
        let (module, marking) = build_module(
            module,
            &module_transitions,
            id as u16,
            &interface_transitions,
        );
        modules.push(module);
        markings.push(marking);
    }

    // find all interface transitions in the modules
    let mut extern_t_overview = vec![vec![]; intern_transition_start as usize];

    for module in &modules {
        for t_id in 0..intern_transition_start {
            let t = &module.transitions[t_id as usize];
            if t.input_places.len() + t.output_places.len() > 0 {
                // is existent in this module
                extern_t_overview[t_id as usize].push(module.id);
            }
        }
    }

    ModularPetrinet {
        modules,
        markings,
        intern_transition_start,
        extern_t_overview,
    }
}

fn build_module(
    m: &RawParserModule,
    t: &Vec<Vec<&RawParserTransition>>,
    id: u16,
    it: &Vec<(String, u32)>,
) -> (PetrinetModul, Marking) {
    // println!("Build Moule: {}", id);
    let mut places = vec![];
    let mut marking = Marking {
        place_counts: vec![],
    };
    // collect all places and set ids
    let mut places_id_map = HashMap::with_capacity(m.places.len());
    for (id, place) in m.places.iter().enumerate() {
        places_id_map.insert(place.name.clone(), id);
        // init places
        places.push(Place {
            id: id as u32,
            name: place.name.clone(),
            input_transitions: vec![],
            output_transitions: vec![],
        });

        // fill init_marking
        if place.weight > 0 {
            marking
                .place_counts
                .push(PlaceCount::new(id as u32, place.weight as u32));
        }
    }
    marking.sort();

    let mut transitions = vec![];
    // seed extern transitions
    for i in 0..it.len() {
        transitions.push(Transition {
            id: i as u32,
            name: "".to_string(),
            input_places: vec![],
            output_places: vec![],
        })
    }

    for raw_transition in t[id as usize].iter() {
        // println!("Transition: {}", raw_transition.name);
        // get id
        let id;
        if let Some(x) = it.iter().find(|(n, _)| n == &raw_transition.name) {
            // extern transitions
            id = x.1;
        } else {
            // intern transition
            id = transitions.len() as u32;
        }

        let mut transition = Transition {
            id,
            name: raw_transition.name.clone(),
            input_places: vec![],
            output_places: vec![],
        };

        // add all input_places
        transition.input_places = collect_all_places(&raw_transition.input_places, &places_id_map);
        // add all output_places
        transition.output_places =
            collect_all_places(&raw_transition.output_places, &places_id_map);

        // add to places
        // inpute places
        for (p_id, weight) in transition.input_places.iter() {
            places[*p_id as usize]
                .output_transitions
                .push((id, *weight));
        }

        // output places
        for (p_id, weight) in transition.output_places.iter() {
            places[*p_id as usize].input_transitions.push((id, *weight));
        }

        // append to vector
        if id as usize >= transitions.len() {
            // intern transitions
            transitions.push(transition);
        } else {
            // extern transitions
            transitions[id as usize] = transition;
        }
    }

    (
        PetrinetModul {
            id,
            name: m.name.clone(),
            places,
            transitions,
        },
        marking,
    )
}

fn collect_all_places(
    places: &Vec<RawParserPlace>,
    p_map: &HashMap<String, usize>,
) -> Vec<(u32, Weight)> {
    let mut transition_places = vec![];
    for p in places {
        if let Some(id) = p_map.get(&p.name) {
            transition_places.push((*id as u32, p.weight as Weight));
        }
    }
    // dbg!(&transition_places);
    transition_places
}

/// returns to which specific modules a transition belongs
fn transition_belongs_module(
    transition: &RawParserTransition,
    place_to_module: &HashMap<String, usize>,
) -> HashSet<u32> {
    let mut module_belonging = HashSet::new();
    let places = transition
        .input_places
        .iter()
        .chain(transition.output_places.iter());

    for place in places {
        let module_id = place_to_module.get(&place.name);

        match module_id {
            Some(x) => {
                module_belonging.insert(*x as u32);
            }
            None => {
                panic!("Place: {} belongs to no module", place.name)
            }
        }
    }
    module_belonging
}
