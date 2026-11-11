#[derive(miresidian::NodeEnum)]
struct NotEnum;

#[derive(miresidian::NodeEnum)]
enum Empty {}

#[derive(miresidian::NodeEnum)]
enum Generic<T> { Item(T) }

#[derive(miresidian::NodeEnum)]
enum Data { Item(u32) }

#[derive(miresidian::NodeEnum)]
enum Tuple { Item() }

#[derive(miresidian::NodeEnum)]
enum Named { Item { value: u32 } }

fn main() {}
