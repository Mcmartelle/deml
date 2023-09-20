extern crate pest;
//#[macro_use]
extern crate pest_derive;

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
                        Rule::name => println!("name: {:?}", node_part.as_str()),
                        Rule::before => println!("before: {:?}", node_part.as_str()),
                        Rule::after => println!("after: {:?}", node_part.as_str()),
                        Rule::command => println!("command: {:?}", node_part.as_str()),
                        _ => unreachable!(),
                    }
                }
            }
            Rule::shelf => println!("shelf: {:?}", part.as_str()),
            Rule::EOI => println!("End of File"),
            _ => unreachable!(),
        }
    }
    Ok(())
}
