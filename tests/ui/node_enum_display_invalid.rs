#[derive(miresidian::NodeEnum)]
#[node_enum("错误位置")]
enum EnumLevel { Item }

#[derive(miresidian::NodeEnum)]
enum Empty { #[node_enum("")] Item }

#[derive(miresidian::NodeEnum)]
enum Blank { #[node_enum(display = " \t")] Item }

#[derive(miresidian::NodeEnum)]
enum Unknown { #[node_enum(label = "显示")] Item }

#[derive(miresidian::NodeEnum)]
enum NonString { #[node_enum(display = 1)] Item }

#[derive(miresidian::NodeEnum)]
enum Duplicate { #[node_enum("一")] #[node_enum(display = "二")] Item }

#[derive(miresidian::NodeEnum)]
enum RepeatedKey { #[node_enum(display = "一", display = "二")] Item }

#[derive(miresidian::NodeEnum)]
enum Mixed { #[node_enum("一", display = "二")] Item }

#[derive(miresidian::NodeEnum)]
enum Extra { #[node_enum("一", "二")] Item }

#[derive(miresidian::NodeEnum)]
enum Missing { #[node_enum()] Item }

fn main() {}
