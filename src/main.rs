extern crate pest;
//#[macro_use]
extern crate pest_derive;

use anyhow::Result;
use clap::Parser as ClapParser;
use dagrs::{
    log, Action, CommandAction, Dag, FileContentError, LogLevel, Parser as DagrsParser,
    ParserError, Task,
};
use pest::iterators::Pair;
use pest::Parser as PestParser;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::{Debug, Display, Formatter};
use std::fs;
use std::sync::Arc;

#[derive(pest_derive::Parser)]
#[grammar = "deml.pest"]
pub struct DagParser;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    dag_file: OsString,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    // println!("Hello DAG path: {:?}", args.dag_file);
    let dag_string = fs::read_to_string(args.dag_file)?;
    let _initialized = log::init_logger(LogLevel::Debug, None);
    let mut dag =
        Dag::with_config_file_and_parser(&dag_string, Box::new(DagFileParser), HashMap::new())?;
    assert!(dag.start()?);
    Ok(())
}

#[derive(Clone)]
struct MyTask {
    tid: usize,
    name: String,
    elevation: isize,
    precursors: HashSet<String>,
    percursor_ids: Vec<usize>,
    postcursors: HashSet<String>,
    postcursors_id: Vec<usize>,
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
            postcursors_id: Vec::new(),
            action: Arc::new(action),
        }
    }

    pub fn init_precursors(&mut self, pres_id: Vec<usize>) {
        self.percursor_ids = pres_id;
    }

    pub fn str_precursors(&self) -> HashSet<String> {
        self.precursors.clone()
    }

    pub fn init_postcursors(&mut self, posts_id: Vec<usize>) {
        self.postcursors_id = posts_id;
    }

    pub fn str_postcursors(&self) -> HashSet<String> {
        self.postcursors.clone()
    }

    pub fn str_id(&self) -> String {
        self.name.clone()
    }
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

impl DagFileParser {
    fn parse_node(&self, node: Pair<'_, Rule>, elevation: isize) -> MyTask {
        // println!("node: {:?}", node.as_str());
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
                    // println!("before_nodes: {:?}", node_part.as_str());
                    for before_node in node_part.into_inner() {
                        match before_node.as_rule() {
                            Rule::before_name => {
                                // println!("before_name: {}", before_node.as_str());
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
                    // println!("after_nodes: {:?}", node_part.as_str());
                    for after_node in node_part.into_inner() {
                        match after_node.as_rule() {
                            Rule::after_name => {
                                // println!("after_name: {}", after_node.as_str());
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
                    println!("command: {:?}", node_part.as_str());
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
            CommandAction::new(command.unwrap()),
        )
    }
}

impl DagrsParser for DagFileParser {
    fn parse_tasks(
        &self,
        file: &str,
        _specific_actions: HashMap<String, Arc<dyn Action + Send + Sync + 'static>>,
    ) -> Result<Vec<Box<dyn Task>>, ParserError> {
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
                    // println!("node: {:?}", part.as_str());
                    let _task = self.parse_node(part, elevation);
                    map_name_to_id.insert(_task.name.clone(), _task.id());
                    println!("node name: {}", _task.name);
                    tasks.insert(_task.name.clone(), _task);
                }
                Rule::shelf => {
                    println!("shelf: {:?}", elevation);
                    elevation -= 1;
                }
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
                | Rule::COMMENT
                | Rule::dag
                | Rule::dag_file => {}
            }
        }
        // Adding nodes with postcursors as precursors to those nodes
        for (name, task) in tasks.clone().iter() {
            for successor in task.postcursors.iter() {
                println!("node: {}, with successor: {}", name, successor);
                let needs_predecessor = tasks.get_mut(successor).expect("successor doesn't exist");
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
            println!("{}", task);
        }
        // panic!()
        Ok(tasks
            .into_iter()
            .map(|(_, task)| Box::new(task) as Box<dyn Task>)
            .collect())
    }
}
