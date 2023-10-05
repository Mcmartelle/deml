extern crate pest;
//#[macro_use]
extern crate pest_derive;

use anyhow::{bail, Result};
use clap::{Args, Parser as ClapParser, Subcommand};
use dagrs::{
    log, Action, CommandAction, Dag, FileContentError, LogLevel, Parser as DagrsParser,
    ParserError, Task,
};
use pest::iterators::Pair;
use pest::Parser as PestParser;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::fs;
use std::io;
use std::sync::Arc;

#[derive(pest_derive::Parser)]
#[grammar = "deml.pest"]
pub struct DagParser;

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a DEML file with dag-rs
    Run(Run),
    /// Convert a DEML file to a Mermaid JS file
    Mermaid(Mermaid),
}

#[derive(Args)]
struct Run {
    /// Path to the input DEML file to run with dagrs. DEML formated strings can also be piped into stdin instead.
    #[arg(short, long)]
    input: Option<String>,
}

#[derive(Args)]
struct Mermaid {
    /// Path to the input DEML file to convert to a Mermaid JS file. DEML formated strings can also be piped into stdin instead.
    #[arg(short, long)]
    input: Option<String>,
    /// Path to the new mermaid file to create, if no output path is given the Mermaid JS contents will go to stdout
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Run(args) => match args.input.clone() {
            Some(input) => {
                let dag_string = fs::read_to_string(input)?;
                run_dag(&dag_string)?;
            }
            None => match io::read_to_string(io::stdin()) {
                Ok(dag_string) => {
                    run_dag(&dag_string)?;
                }
                Err(_) => {
                    bail!("no input provided, use stdin or -i <filepath>")
                }
            },
        },
        Commands::Mermaid(args) => match args.input.clone() {
            Some(input) => {
                let dag_string = fs::read_to_string(input)?;
                mermaid_dag(&dag_string, args)?;
            }
            None => match io::read_to_string(io::stdin()) {
                Ok(dag_string) => {
                    mermaid_dag(&dag_string, args)?;
                }
                Err(_) => {
                    bail!("no input provided, use stdin or -i <filepath>")
                }
            },
        },
    }

    Ok(())
}

fn run_dag(dag_string: &str) -> Result<()> {
    let _initialized = log::init_logger(LogLevel::Debug, None);
    let mut dag =
        Dag::with_config_file_and_parser(dag_string, Box::new(DagFileParser), HashMap::new())?;
    assert!(dag.start()?);
    Ok(())
}

fn mermaid_dag(dag_string: &str, args: &Mermaid) -> Result<(), anyhow::Error> {
    let tasks = parse_dag(dag_string)?;
    let mermaid_string = tasks_to_mermaid(tasks)?;
    match &args.output {
        Some(output) => match fs::File::open(&output) {
            Ok(_) => {
                bail!("output file {} already exists", output);
            }
            Err(_) => {
                fs::write(output, mermaid_string)?;
            }
        },
        None => {
            println!("{}", mermaid_string);
        }
    }
    Ok(())
}

fn tasks_to_mermaid(tasks: HashMap<String, MyTask>) -> Result<String, anyhow::Error> {
    let mut mermaid_string = String::new();
    mermaid_string.push_str("flowchart TD\n");
    for (name, task) in tasks.iter() {
        if task.precursors.is_empty() {
            continue;
        }
        mermaid_string.push_str("    "); // four spaces
        let mut precursor_iter = task.precursors.iter().peekable();
        while let Some(precursor) = precursor_iter.next() {
            mermaid_string.push_str(precursor);
            if precursor_iter.peek() != None {
                mermaid_string.push_str(" & ");
            }
        }
        mermaid_string.push_str(" ---> ");
        mermaid_string.push_str(name);
        mermaid_string.push_str("\n");
    }
    Ok(mermaid_string)
}

#[derive(Clone)]
struct MyTask {
    tid: usize,
    name: String,
    elevation: isize,
    precursors: HashSet<String>,
    percursor_ids: Vec<usize>,
    postcursors: HashSet<String>,
    // postcursors_id: Vec<usize>,
    action: Arc<dyn Action + Sync + Send>,
}

impl MyTask {
    pub fn new(
        name: String,
        elevation: isize,
        precursors: HashSet<String>,
        postcursors: HashSet<String>,
        action: impl Action + Send + Sync + 'static,
    ) -> Self {
        Self {
            tid: dagrs::alloc_id(),
            name,
            elevation,
            precursors,
            percursor_ids: Vec::new(),
            postcursors,
            // postcursors_id: Vec::new(),
            action: Arc::new(action),
        }
    }

    pub fn init_precursors(&mut self, pres_id: Vec<usize>) {
        self.percursor_ids = pres_id;
    }

    // pub fn str_precursors(&self) -> HashSet<String> {
    //     self.precursors.clone()
    // }

    // pub fn init_postcursors(&mut self, posts_id: Vec<usize>) {
    //     self.postcursors_id = posts_id;
    // }

    // pub fn str_postcursors(&self) -> HashSet<String> {
    //     self.postcursors.clone()
    // }

    // pub fn str_id(&self) -> String {
    //     self.name.clone()
    // }
}

impl Task for MyTask {
    fn action(&self) -> Arc<dyn Action + Sync + Send> {
        self.action.clone()
    }
    fn predecessors(&self) -> &[usize] {
        &self.percursor_ids
    }
    fn id(&self) -> usize {
        self.tid
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

impl Display for MyTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},pre:{:?},post:{:?}",
            self.name, self.tid, self.precursors, self.postcursors
        )
    }
}

struct DagFileParser;

fn parse_node(node: Pair<'_, Rule>, elevation: isize) -> MyTask {
    let mut node_name: &str = "";
    let mut precursors: HashSet<String> = HashSet::new();
    let mut postcursors: HashSet<String> = HashSet::new();
    let mut command: Option<&str> = None;
    for node_part in node.into_inner() {
        match node_part.as_rule() {
            Rule::node_name => {
                node_name = node_part.as_str().trim_end();
            }
            Rule::before_nodes => {
                for before_node in node_part.into_inner() {
                    match before_node.as_rule() {
                        Rule::before_name => {
                            precursors.insert(String::from(before_node.as_str().trim_end()));
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
                        | Rule::COMMENT
                        | Rule::dag
                        | Rule::dag_file
                        | Rule::EOI => {}
                    }
                }
            }
            Rule::after_nodes => {
                for after_node in node_part.into_inner() {
                    match after_node.as_rule() {
                        Rule::after_name => {
                            postcursors.insert(String::from(after_node.as_str().trim_end()));
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
                        | Rule::COMMENT
                        | Rule::dag
                        | Rule::dag_file
                        | Rule::EOI => {}
                    }
                }
            }
            Rule::command => {
                command = Some(node_part.as_str());
            }
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
            | Rule::COMMENT
            | Rule::char
            | Rule::EOI => {}
        }
    }
    MyTask::new(
        String::from(node_name),
        elevation,
        precursors,
        postcursors,
        CommandAction::new(command.unwrap_or(":")),
    )
}

fn parse_dag(file: &str) -> Result<HashMap<String, MyTask>, ParserError> {
    let dag_parts = DagParser::parse(Rule::dag, file)
        .expect("unsuccessful pest parse")
        .next()
        .unwrap();
    let mut map_name_to_id: HashMap<String, usize> = HashMap::new();
    let mut tasks: HashMap<String, MyTask> = HashMap::new();
    let mut elevation: isize = 0;

    for part in dag_parts.into_inner() {
        match part.as_rule() {
            Rule::node => {
                let _task = parse_node(part, elevation);
                map_name_to_id.insert(_task.name.clone(), _task.id());
                tasks.insert(_task.name.clone(), _task);
            }
            Rule::shelf => {
                elevation -= 1;
            }
            Rule::EOI
            | Rule::node_name
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
            | Rule::COMMENT
            | Rule::dag
            | Rule::dag_file => {}
        }
    }
    // Adding nodes with postcursors as precursors to those nodes
    for (name, task) in tasks.clone().iter() {
        for successor in task.postcursors.iter() {
            let needs_predecessor = match tasks.get_mut(successor) {
                Some(x) => x,
                None => {
                    let msg = String::from("successor: '")
                        + successor
                        + "' for node: '"
                        + name
                        + "' not found.";
                    return Err(ParserError::FileContentError(FileContentError::Empty(msg)));
                }
            };
            if task.elevation <= needs_predecessor.elevation {
                let msg: String = String::from("The file isn't empty but the node ")
                    + name
                    + " has an elevation of "
                    + task.elevation.to_string().as_str()
                    + " which needs to be greater than its successor's elevation which is "
                    + needs_predecessor.elevation.to_string().as_str()
                    + " for successor node "
                    + needs_predecessor.name.to_string().as_str();
                return Err(ParserError::FileContentError(FileContentError::Empty(msg)));
            }
            needs_predecessor.precursors.insert(task.name.clone());
        }
    }
    // Ensuring precursors are at a higher elevation than the nodes they preceed
    for (name, task) in tasks.iter() {
        for precursor in task.precursors.iter() {
            let precursor_task = tasks.get(precursor).expect("precursor doesn't exist");
            if task.elevation >= precursor_task.elevation {
                let msg: String = String::from("The file isn't empty but the node ")
                    + name
                    + " has an elevation of "
                    + task.elevation.to_string().as_str()
                    + " which needs to be less than its precursor's elevation which is "
                    + precursor_task.elevation.to_string().as_str()
                    + " for precursor node "
                    + precursor_task.name.to_string().as_str();
                return Err(ParserError::FileContentError(FileContentError::Empty(msg)));
            }
        }
    }
    for (_, task) in tasks.iter_mut() {
        let mut pre_ids: Vec<usize> = Vec::new();
        for precursor in task.precursors.iter() {
            pre_ids.push(
                *map_name_to_id
                    .get(precursor)
                    .expect("precursor node does not exist"),
            );
        }
        task.init_precursors(pre_ids);
    }

    Ok(tasks)
}

impl DagrsParser for DagFileParser {
    fn parse_tasks(
        &self,
        file: &str,
        _specific_actions: HashMap<String, Arc<dyn Action + Send + Sync + 'static>>,
    ) -> Result<Vec<Box<dyn Task>>, ParserError> {
        let tasks = parse_dag(file)?;
        Ok(tasks
            .into_iter()
            .map(|(_, task)| Box::new(task) as Box<dyn Task>)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_deml() {
        let dag_string = "UpRiver > A\n----\nA > B\n----\nB > C | D | E\n----\nC\nD\nE\n----\nF < C\nG < D | E > DownRiver\n----\nDownRiver < F";
        let mut dag = match parse_dag(dag_string) {
            Ok(dag) => dag,
            Err(e) => panic!("parse_dag has an error: {}", e),
        };
        assert!(dag.contains_key("UpRiver"));
        assert!(dag.contains_key("A"));
        assert!(dag.contains_key("B"));
        assert!(dag.contains_key("C"));
        assert!(dag.contains_key("D"));
        assert!(dag.contains_key("E"));
        assert!(dag.contains_key("F"));
        assert!(dag.contains_key("G"));
        assert!(dag.contains_key("DownRiver"));
        let node = dag.get_mut("UpRiver").unwrap();
        assert!(node.precursors.is_empty());
        let node = dag.get_mut("A").unwrap();
        assert!(node.precursors.contains("UpRiver"));
        let node = dag.get_mut("B").unwrap();
        assert!(node.precursors.contains("A"));
        let node = dag.get_mut("C").unwrap();
        assert!(node.precursors.contains("B"));
        let node = dag.get_mut("D").unwrap();
        assert!(node.precursors.contains("B"));
        let node = dag.get_mut("E").unwrap();
        assert!(node.precursors.contains("B"));
        let node = dag.get_mut("F").unwrap();
        assert!(node.precursors.contains("C"));
        let node = dag.get_mut("G").unwrap();
        assert!(node.precursors.contains("D"));
        assert!(node.precursors.contains("E"));
        let node = dag.get_mut("DownRiver").unwrap();
        assert!(node.precursors.contains("F"));
        assert!(node.precursors.contains("G"));
    }
}
