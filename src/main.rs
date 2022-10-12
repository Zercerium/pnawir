use std::fs::{self};

use clap::Parser;

use pnawir::{
    self, parser::transform_input,
    sync_reachability_graph::build_graph::build_sync_reachability_graph,
};
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]

struct Cli {
    #[arg(short, long, value_parser)]
    filename: String,
}

fn main() {
    let s_def = include_str!("../examples/ba2022/P002.pnawir");

    let args = Cli::parse();

    let input;
    match fs::read_to_string(&args.filename) {
        Ok(s) => input = s,
        Err(_ee) => {
            println!("File: {} not found", &args.filename);
            println!("using default template");
            input = s_def.to_string();
            // panic!("{}", _e);
        }
    }

    // parse net
    let (_, raw_parser_input) = pnawir::parser::parse_input::parse(&input[..]).unwrap();
    // dbg!(&raw_parser_input);
    let modular_net = transform_input::transform(raw_parser_input);
    // dbg!(&modular_net);

    let graph = build_sync_reachability_graph(&modular_net);
    graph.print(&modular_net);
}
