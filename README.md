# DEML (DAG Elevation Markup Language)
## Warning: Experimental

Languages designed to represent all types of graph data structures, such as Graphviz's [DOT Language](https://graphviz.org/doc/info/lang.html) and Mermaid JS's [flowchart syntax](https://mermaid.js.org/syntax/flowchart.html), don't take advantage of the properties specific to DAGs ([Directed Acyclic Graphs](https://en.wikipedia.org/wiki/Directed_acyclic_graph)).

DAGs act like rivers. Water doesn't flow upstream (tides and floods being exceptions). Sections of a river at the same elevation can't be the inputs or outputs of each other, like the nodes C, D, and E in the image below. Their input is B. C outputs to F, while D and E output to G.


![Photo of a river to illustrate how DAGs operate](assets/river-example.jpg)

DEML's goal is to use this ordering in the file syntax to make it easier for humans to parse. In DEML we represent an elevation marker with `----` on a new line. The order of elevation clusters is significant, but the order of nodes between two `----` elevation markers is not significant.

```Haskell
UpRiver > A
----
A > B
----
B > C | D | E
----
C
D
E
----
F < C
G < D | E > DownRiver
----
DownRiver < F
```

Nodes are defined by the first word on a line. The defined node can point to its outputs with `>` and to its inputs with `<`. Inputs and outputs are separated by `|`. 

## DAG-RS

The [DAG-RS YAML example](https://github.com/open-rust-initiative/dagrs#yaml-configuration-file) for running shell commands in a DAG defined order.
```YAML
dagrs:
  a:
    name: "Task 1"
    after: [ b, c ]
    cmd: echo a
  b:
    name: "Task 2"
    after: [ c, f, g ]
    cmd: echo b
  c:
    name: "Task 3"
    after: [ e, g ]
    cmd: echo c
  d:
    name: "Task 4"
    after: [ c, e ]
    cmd: echo d
  e:
    name: "Task 5"
    after: [ h ]
    cmd: echo e
  f:
    name: "Task 6"
    after: [ g ]
    cmd: python3 ./tests/config/test.py
  g:
    name: "Task 7"
    after: [ h ]
    cmd: node ./tests/config/test.js
  h:
    name: "Task 8"
    cmd: echo h
```

 Would be represented in DEML as follows

```Haskell
H > E | G = echo h
----
G = node ./tests/config/test.js
G = echo e
----
F < G = python3 ./tests/config/test.py
C < E | G = echo c
----
B < C | F | G = echo b
D < C | E = echo d
----
A < B | C = echo a
```

Shell commands can be assigned to a node with `=`. DEML files can be run with dag-rs with the comand `deml run -i <filepath>`.

## Mermaid JS

To convert to Mermaid Diagram files (.mmd) use the command `deml mermaid -i <inputfile> -o <outputfile>`. The mermaid file can be used to generate an image at [mermaid.live](https://mermaid.live/)

![mermaid js flowchart image of the river DAG](assets/river-mermaid-diagram.png)

## Goals

- [x] Put my idea for an elevation based DAG representation into the wild
- [x] Run DAGs with dag-rs
- [x] Convert DEML files to Mermaid Diagram files 
- [ ] Add a syntax to label edges

## Possible Goals
- [ ] Syntax highlighting (Haskell syntax highlighting works well enough for the examples in this README)

## Non-Goals

- Supporting commercial products

## Why, Though?

I was thinking about how it's annoying in languages like C when function declaration order matters. Then I wondered if there would be a case when it would be a nice feature for declaration order to matter and I thought of DAGs.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
