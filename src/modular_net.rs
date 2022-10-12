use crate::sync_reachability_graph::graph::Marking;

pub type TransitionId = u32;
pub type PlaceId = u32;
pub type ModuleId = u16;
pub type Weight = u32;

#[derive(Debug)]
pub struct ModularPetrinet {
    pub modules: Vec<PetrinetModul>,
    pub markings: Vec<Marking>,
    pub intern_transition_start: u32,
    pub extern_t_overview: Vec<Vec<ModuleId>>,
}

#[derive(Debug)]
pub struct PetrinetModul {
    pub id: ModuleId,
    pub name: String,
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub id: PlaceId,
    pub name: String,
    pub input_transitions: Vec<(TransitionId, Weight)>,
    pub output_transitions: Vec<(TransitionId, Weight)>,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub id: TransitionId,
    pub name: String,
    pub input_places: Vec<(PlaceId, Weight)>,
    pub output_places: Vec<(PlaceId, Weight)>,
}
