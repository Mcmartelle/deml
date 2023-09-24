extern crate pest;
//#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate icecream;

use anyhow::Result;
use clap::Parser as ClapParser;
use pest::Parser as PestParser;
use std::ffi::OsString;
use std::fs;

#[derive(pest_derive::Parser)]
#[grammar = "dag.pest"]
pub struct DagParser;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    dag_file: OsString,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    println!("Hello DAG path: {:?}", args.dag_file);
    let dag_string = fs::read_to_string(args.dag_file)?;
    parse_dag(&dag_string)?;
    Ok(())
}

fn parse_dag(dag_string: &String) -> Result<()> {
    let dag_parts = DagParser::parse(Rule::dag, &dag_string)
        .expect("unsuccessful parse")
        .next()
        .unwrap();
    for part in dag_parts.into_inner() {
        match part.as_rule() {
            Rule::node => {
                println!("node: {:?}", part.as_str());
                for node_part in part.into_inner() {
                    match node_part.as_rule() {
                        Rule::node_name => {
                            ic!("node name:", node_part.as_str());
                        }
                        Rule::before_nodes => {
                            println!("before_nodes: {:?}", node_part.as_str());
                            for before_node in node_part.into_inner() {
                                match before_node.as_rule() {
                                    Rule::before_name => {
                                        println!("before_name: {}", before_node.as_str());
                                    }
                                    Rule::node
                                    | Rule::shelf
                                    | Rule::node_name
                                    | Rule::before
                                    | Rule::after
                                    | Rule::after_name
                                    | Rule::before_nodes
                                    | Rule::after_nodes
                                    | Rule::command
                                    | Rule::name
                                    | Rule::char
                                    | Rule::WHITESPACE
                                    | Rule::dag
                                    | Rule::dag_file
                                    | Rule::EOI => {}
                                }
                            }
                        }
                        Rule::after_nodes => {
                            println!("after_nodes: {:?}", node_part.as_str());
                            for after_node in node_part.into_inner() {
                                match after_node.as_rule() {
                                    Rule::after_name => {
                                        println!("after_name: {}", after_node.as_str());
                                    }
                                    Rule::node
                                    | Rule::shelf
                                    | Rule::node_name
                                    | Rule::before
                                    | Rule::after
                                    | Rule::before_name
                                    | Rule::before_nodes
                                    | Rule::after_nodes
                                    | Rule::command
                                    | Rule::name
                                    | Rule::char
                                    | Rule::WHITESPACE
                                    | Rule::dag
                                    | Rule::dag_file
                                    | Rule::EOI => {}
                                }
                            }
                        }
                        Rule::command => println!("command: {:?}", node_part.as_str()),
                        Rule::shelf
                        | Rule::node
                        | Rule::dag
                        | Rule::dag_file
                        | Rule::name
                        | Rule::before
                        | Rule::after
                        | Rule::before_name
                        | Rule::after_name
                        | Rule::WHITESPACE
                        | Rule::char
                        | Rule::EOI => {}
                    }
                }
            }
            Rule::shelf => println!("shelf: {:?}", part.as_str()),
            Rule::EOI => println!("End of File"),
            Rule::node_name
            | Rule::before
            | Rule::after
            | Rule::before_name
            | Rule::after_name
            | Rule::before_nodes
            | Rule::after_nodes
            | Rule::command
            | Rule::name
            | Rule::char
            | Rule::WHITESPACE
            | Rule::dag
            | Rule::dag_file => {}
        }
    }
    Ok(())
}
