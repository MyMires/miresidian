use miresidian_macros::node_impl;

trait ExternalContract {}
struct TraitNode;

#[node_impl]
impl ExternalContract for TraitNode {}

fn main() {}
