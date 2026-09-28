use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::sync::{Mutex, OnceLock};

const MAX_NODES: usize = 120;
const NODE_HEIGHT: f64 = 64.0;
const RANK_GAP: f64 = 42.0;
const NODE_GAP: f64 = 28.0;
const MARGIN: f64 = 58.0;
const MAX_LAYOUT_WIDTH: f64 = 1400.0;
const ROW_GAP: f64 = 96.0;
const BRANCH_GAP: f64 = 72.0;
const GROUP_BLOCK_GAP: f64 = 44.0;
const PROCESS_CORNER_RADIUS: f64 = 12.0;
const GROUP_CORNER_RADIUS: f64 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    LeftToRight,
    TopDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeKind {
    Process,
    Decision,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeTone {
    Purple,
    Orange,
    Green,
}

#[derive(Clone, Debug)]
struct Node {
    id: String,
    label: String,
    return_target: Option<String>,
    kind: NodeKind,
    tone: NodeTone,
    is_rework: bool,
    group: Option<String>,
    source_order: usize,
    rank: usize,
    order: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Debug)]
struct Edge {
    from: usize,
    to: usize,
    label: String,
    dashed: bool,
    arrow: bool,
    is_return: bool,
    source_order: usize,
}

#[derive(Clone, Debug)]
struct Group {
    id: String,
    label: String,
    parent: Option<String>,
    direction: Option<Direction>,
    source_order: usize,
}

#[derive(Debug)]
struct Graph {
    direction: Direction,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    groups: Vec<Group>,
}

#[derive(Default)]
struct OutputState {
    bytes: Vec<u8>,
    ok: bool,
}

fn output_state() -> &'static Mutex<OutputState> {
    static OUTPUT: OnceLock<Mutex<OutputState>> = OnceLock::new();
    OUTPUT.get_or_init(|| Mutex::new(OutputState::default()))
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(length: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(length.max(1));
    let pointer = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    pointer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(pointer: *mut u8, capacity: usize) {
    if !pointer.is_null() {
        // SAFETY: JavaScript returns the exact pointer and capacity allocated by `alloc`.
        unsafe { drop(Vec::from_raw_parts(pointer, 0, capacity.max(1))) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn render(pointer: *const u8, length: usize) -> i32 {
    let result = if pointer.is_null() {
        Err("Mermaid source pointer is null".to_string())
    } else {
        // SAFETY: the caller writes exactly `length` initialized bytes into `alloc`'s buffer.
        let input = unsafe { std::slice::from_raw_parts(pointer, length) };
        match std::str::from_utf8(input) {
            Ok(source) => parse_graph(source).and_then(|mut graph| {
                layout_graph(&mut graph)?;
                Ok(render_svg(&graph, source))
            }),
            Err(_) => Err("Mermaid source must be valid UTF-8".to_string()),
        }
    };

    let mut state = output_state().lock().expect("output mutex poisoned");
    match result {
        Ok(svg) => {
            state.bytes = svg.into_bytes();
            state.ok = true;
            1
        }
        Err(error) => {
            state.bytes = error.into_bytes();
            state.ok = false;
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn result_ptr() -> *const u8 {
    output_state()
        .lock()
        .expect("output mutex poisoned")
        .bytes
        .as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize {
    output_state()
        .lock()
        .expect("output mutex poisoned")
        .bytes
        .len()
}

#[unsafe(no_mangle)]
pub extern "C" fn last_ok() -> i32 {
    i32::from(output_state().lock().expect("output mutex poisoned").ok)
}

fn parse_graph(source: &str) -> Result<Graph, String> {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.lines().enumerate().peekable();
    let mut direction = None;
    let mut body = Vec::new();

    while let Some((line_number, line)) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        let mut words = trimmed.split_whitespace();
        let keyword = words.next().unwrap_or_default().to_ascii_lowercase();
        if keyword != "flowchart" && keyword != "graph" {
            return Err(format!(
                "Only standard Mermaid flowchart/graph LR, TD and TB are handled (line {})",
                line_number + 1
            ));
        }
        direction = match words
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase()
            .as_str()
        {
            "LR" => Some(Direction::LeftToRight),
            "TD" | "TB" => Some(Direction::TopDown),
            other => {
                return Err(format!(
                    "Direction {other} is not handled; native Mermaid retained"
                ));
            }
        };
        body.extend(lines);
        break;
    }

    let direction = direction.ok_or_else(|| "Missing flowchart/graph header".to_string())?;
    let mut nodes = Vec::<Node>::new();
    let mut node_lookup = HashMap::<String, usize>::new();
    let mut edges = Vec::<Edge>::new();
    let mut groups = Vec::<Group>::new();
    let mut group_stack = Vec::<String>::new();
    let mut class_assignments = Vec::<(Vec<String>, String)>::new();

    for (line_number, raw_line) in body {
        let mut line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        let lower = line.to_ascii_lowercase();
        if lower.starts_with("subgraph ") {
            let spec = line[9..].trim();
            let (id, label, _) = parse_node_spec(spec, groups.len())?;
            let group_id = if id.is_empty() {
                format!("group_{}", groups.len())
            } else {
                id
            };
            groups.push(Group {
                id: group_id.clone(),
                label: if label.is_empty() {
                    group_id.clone()
                } else {
                    label
                },
                parent: group_stack.last().cloned(),
                direction: None,
                source_order: groups.len(),
            });
            group_stack.push(group_id);
            continue;
        }
        if lower == "end" {
            group_stack.pop();
            continue;
        }
        if lower.starts_with("class ") {
            if let Some(assignment) = parse_class_statement(line) {
                class_assignments.push(assignment);
            }
            continue;
        }
        if lower.starts_with("direction ") {
            if let Some(group_id) = group_stack.last()
                && let Some(group) = groups.iter_mut().find(|group| &group.id == group_id)
            {
                let value = line["direction".len()..].trim().to_ascii_uppercase();
                group.direction = match value.as_str() {
                    "LR" => Some(Direction::LeftToRight),
                    "TD" | "TB" => Some(Direction::TopDown),
                    _ => None,
                };
            }
            continue;
        }
        if lower.starts_with("classdef ")
            || lower.starts_with("style ")
            || lower.starts_with("linkstyle ")
            || lower.starts_with("click ")
            || lower.starts_with("accdescr")
            || lower.starts_with("acctitle")
        {
            continue;
        }

        if let Some(parsed_edges) = parse_edge_chain(line) {
            for (left, right, label, dashed, arrow) in parsed_edges {
                let from = add_node(
                    &mut nodes,
                    &mut node_lookup,
                    left,
                    group_stack.last().cloned(),
                )?;
                let to = add_node(
                    &mut nodes,
                    &mut node_lookup,
                    right,
                    group_stack.last().cloned(),
                )?;
                edges.push(Edge {
                    from,
                    to,
                    label: clean_label(&label),
                    dashed,
                    arrow,
                    is_return: false,
                    source_order: edges.len(),
                });
            }
            continue;
        }

        if looks_like_node(line) {
            add_node(
                &mut nodes,
                &mut node_lookup,
                line,
                group_stack.last().cloned(),
            )?;
            continue;
        }

        // Mermaid supports many optional statements. Unknown non-structural statements are
        // deliberately rejected so Typora can render them with its native engine instead.
        line = line.trim();
        return Err(format!(
            "Unsupported Mermaid statement on line {}: {}",
            line_number + 1,
            truncate(line, 72)
        ));
    }

    if nodes.is_empty() {
        return Err("The flowchart contains no nodes".to_string());
    }
    if nodes.len() > MAX_NODES {
        return Err(format!(
            "The flowchart has {} nodes; the local renderer limit is {MAX_NODES}",
            nodes.len()
        ));
    }

    for (ids, class_name) in class_assignments {
        for id in ids {
            if let Some(index) = node_lookup.get(&id).copied() {
                apply_style_class(&mut nodes[index], &class_name);
            }
        }
    }

    // A rework node may contain a textual note such as “返回：收集需求”. That
    // note describes a relationship which the renderer now draws as a real
    // dashed return edge, so keep only the node's actual action label.
    for node in &mut nodes {
        if is_rework_node(node) {
            let original_label = node.label.clone();
            let (label, return_target) = strip_rework_return_annotation(&original_label);
            node.return_target = return_target;
            if label != original_label {
                node.label = label;
                let (width, height) = measure_node(&node.label, node.kind);
                node.width = width;
                node.height = height;
            }
        }
    }

    Ok(Graph {
        direction,
        nodes,
        edges,
        groups,
    })
}

type ParsedEdgeSpec<'a> = (&'a str, &'a str, String, bool, bool);

fn parse_edge_chain(line: &str) -> Option<Vec<ParsedEdgeSpec<'_>>> {
    let first_operator = find_edge_operator_start(line)?;
    let mut left = line[..first_operator].trim();
    let mut operator_and_tail = &line[first_operator..];
    let mut parsed = Vec::new();

    loop {
        let (consumed, mut label, dashed, arrow) = parse_edge_operator(operator_and_tail)?;
        let mut tail = operator_and_tail[consumed..].trim_start();
        if let Some(after_pipe) = tail.strip_prefix('|') {
            let pipe_end = after_pipe.find('|')?;
            label = after_pipe[..pipe_end].trim().to_string();
            tail = after_pipe[pipe_end + 1..].trim_start();
        }

        let next_operator = find_edge_operator_start(tail);
        let right = next_operator
            .map_or(tail, |position| &tail[..position])
            .trim();
        if left.is_empty() || right.is_empty() {
            return None;
        }
        parsed.push((left, right, label, dashed, arrow));

        let Some(position) = next_operator else {
            break;
        };
        left = right;
        operator_and_tail = &tail[position..];
    }

    Some(parsed)
}

fn find_edge_operator_start(line: &str) -> Option<usize> {
    let mut candidates = [" -- ", " -.", "-.->", "==>", "-->", "---"]
        .into_iter()
        .filter_map(|needle| find_outside_delimiters(line, needle))
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    candidates
        .into_iter()
        .find(|&position| parse_edge_operator(&line[position..]).is_some())
}

fn parse_edge_operator(input: &str) -> Option<(usize, String, bool, bool)> {
    // Mermaid's label-between-dashes form: A -- approved --> B
    if let Some(remainder) = input.strip_prefix(" -- ")
        && let Some(end) = find_outside_delimiters(remainder, " -->")
    {
        let consumed = " -- ".len() + end + " -->".len();
        return Some((consumed, remainder[..end].trim().to_string(), false, true));
    }

    // Dotted label form: A -. retry .-> B
    if let Some(remainder) = input.strip_prefix(" -.")
        && !remainder.starts_with("->")
        && let Some(end) = find_outside_delimiters(remainder, ".->")
    {
        let consumed = " -.".len() + end + ".->".len();
        return Some((consumed, remainder[..end].trim().to_string(), true, true));
    }

    for (operator, dashed, arrow) in [
        ("-.->", true, true),
        ("==>", false, true),
        ("-->", false, true),
        ("---", false, false),
    ] {
        if input.starts_with(operator) {
            return Some((operator.len(), String::new(), dashed, arrow));
        }
    }
    None
}

fn find_outside_delimiters(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let mut square = 0_i32;
    let mut round = 0_i32;
    let mut curly = 0_i32;
    let mut quote = None::<u8>;
    let mut index = 0;
    while index + needle.len() <= bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if byte == active_quote && (index == 0 || bytes[index - 1] != b'\\') {
                quote = None;
            }
        } else {
            match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'[' => square += 1,
                b']' => square = (square - 1).max(0),
                b'(' => round += 1,
                b')' => round = (round - 1).max(0),
                b'{' => curly += 1,
                b'}' => curly = (curly - 1).max(0),
                _ => {}
            }
            if square == 0
                && round == 0
                && curly == 0
                && bytes[index..].starts_with(needle.as_bytes())
            {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn looks_like_node(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && !trimmed.contains(char::is_whitespace)
        || trimmed.contains('[')
        || trimmed.contains('(')
        || trimmed.contains('{')
}

fn add_node(
    nodes: &mut Vec<Node>,
    lookup: &mut HashMap<String, usize>,
    spec: &str,
    group: Option<String>,
) -> Result<usize, String> {
    let (id, label, parsed_kind) = parse_node_spec(spec, nodes.len())?;
    if id.is_empty() {
        return Err(format!(
            "Node is missing an identifier: {}",
            truncate(spec, 72)
        ));
    }
    if let Some(index) = lookup.get(&id).copied() {
        let node = &mut nodes[index];
        if label != id && !label.is_empty() {
            node.label = label;
            node.kind = classify_node(parsed_kind, &node.label);
            node.tone = tone_for_kind(node.kind);
            let (width, height) = measure_node(&node.label, node.kind);
            node.width = width;
            node.height = height;
        }
        if node.group.is_none() {
            node.group = group;
        }
        return Ok(index);
    }

    let kind = classify_node(parsed_kind, &label);
    let tone = tone_for_kind(kind);
    let (width, height) = measure_node(&label, kind);
    let index = nodes.len();
    nodes.push(Node {
        id: id.clone(),
        label,
        return_target: None,
        kind,
        tone,
        is_rework: false,
        group,
        source_order: index,
        rank: 0,
        order: index as f64,
        x: 0.0,
        y: 0.0,
        width,
        height,
    });
    lookup.insert(id, index);
    Ok(index)
}

fn parse_node_spec(
    spec: &str,
    fallback_index: usize,
) -> Result<(String, String, NodeKind), String> {
    let spec = spec
        .trim()
        .trim_end_matches(|character: char| character == ';' || character.is_whitespace());
    let spec = spec.split(":::").next().unwrap_or(spec).trim();
    if spec.is_empty() {
        return Err("Empty node specification".to_string());
    }

    if (spec.starts_with('"') && spec.ends_with('"'))
        || (spec.starts_with('\'') && spec.ends_with('\''))
    {
        let label = clean_label(spec);
        return Ok((format!("node_{fallback_index}"), label, NodeKind::Process));
    }

    let opening = spec
        .char_indices()
        .find(|(_, character)| matches!(character, '[' | '(' | '{' | '>'));
    let Some((position, character)) = opening else {
        let id = sanitize_id(spec);
        return Ok((id.clone(), id, NodeKind::Process));
    };

    let id = sanitize_id(&spec[..position]);
    let remainder = &spec[position..];
    let (raw_label, kind) = match character {
        '{' => (strip_balanced(remainder, '{', '}'), NodeKind::Decision),
        '(' if remainder.starts_with("([") => {
            (strip_wrappers(remainder, "([", "])"), NodeKind::Terminal)
        }
        '(' if remainder.starts_with("((") => {
            (strip_wrappers(remainder, "((", "))"), NodeKind::Terminal)
        }
        '(' => (strip_balanced(remainder, '(', ')'), NodeKind::Process),
        '[' if remainder.starts_with("[[") => {
            (strip_wrappers(remainder, "[[", "]]"), NodeKind::Process)
        }
        '[' => (strip_balanced(remainder, '[', ']'), NodeKind::Process),
        '>' => (strip_balanced(remainder, '>', ']'), NodeKind::Process),
        _ => (remainder, NodeKind::Process),
    };
    let label = clean_label(raw_label);
    Ok((
        if id.is_empty() {
            format!("node_{fallback_index}")
        } else {
            id
        },
        if label.is_empty() {
            format!("node_{fallback_index}")
        } else {
            label
        },
        kind,
    ))
}

fn strip_balanced(value: &str, opening: char, closing: char) -> &str {
    value
        .strip_prefix(opening)
        .and_then(|inner| inner.rfind(closing).map(|end| &inner[..end]))
        .unwrap_or(value)
}

fn strip_wrappers<'a>(value: &'a str, opening: &str, closing: &str) -> &'a str {
    value
        .strip_prefix(opening)
        .and_then(|inner| inner.rfind(closing).map(|end| &inner[..end]))
        .unwrap_or(value)
}

fn sanitize_id(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character: char| character == '"' || character == '\'')
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

fn clean_label(value: &str) -> String {
    let mut label = value
        .trim()
        .trim_matches(|character: char| character == '"' || character == '\'')
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("#quot;", "\"");
    while let Some(start) = label.find('<') {
        let Some(relative_end) = label[start..].find('>') else {
            break;
        };
        label.replace_range(start..=start + relative_end, "");
    }
    label.trim().to_string()
}

fn strip_rework_return_annotation(label: &str) -> (String, Option<String>) {
    let mut kept = Vec::new();
    let mut return_target = None;
    for line in label.lines() {
        let trimmed = line.trim();
        if let Some(target) = parse_return_annotation(trimmed) {
            return_target = Some(target);
        } else if !trimmed.is_empty() {
            kept.push(trimmed);
        }
    }
    let cleaned = if kept.is_empty() {
        label
            .lines()
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    } else {
        kept.join("\n")
    };
    (cleaned, return_target)
}

fn parse_return_annotation(line: &str) -> Option<String> {
    let remainder = line.strip_prefix("返回")?;
    let target = remainder
        .trim_start_matches(|character: char| {
            matches!(character, ':' | '：' | ' ' | '-' | '>' | '→')
        })
        .trim();
    (!target.is_empty()).then(|| target.to_string())
}

fn classify_node(parsed: NodeKind, label: &str) -> NodeKind {
    if parsed == NodeKind::Decision {
        return NodeKind::Decision;
    }
    if parsed == NodeKind::Terminal || is_terminal_label(label) {
        NodeKind::Terminal
    } else {
        NodeKind::Process
    }
}

fn is_terminal_label(label: &str) -> bool {
    let normalized = label
        .trim()
        .trim_matches(|character: char| {
            character.is_ascii_punctuation() || matches!(character, '。' | '，' | '！' | '？')
        })
        .trim()
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "start" | "end" | "finish" | "done" | "开始" | "结束" | "完成"
    )
}

fn tone_for_kind(kind: NodeKind) -> NodeTone {
    match kind {
        NodeKind::Process => NodeTone::Purple,
        NodeKind::Decision => NodeTone::Orange,
        NodeKind::Terminal => NodeTone::Green,
    }
}

fn parse_class_statement(line: &str) -> Option<(Vec<String>, String)> {
    let rest = line.trim().strip_prefix("class")?.trim();
    let mut parts = rest.split_whitespace();
    let class_name = parts.next_back()?.trim().to_string();
    if class_name.is_empty() {
        return None;
    }
    let ids = parts
        .collect::<Vec<_>>()
        .join("")
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    (!ids.is_empty()).then_some((ids, class_name))
}

fn apply_style_class(node: &mut Node, class_name: &str) {
    let normalized = class_name.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "ryen-process" => {
            node.kind = NodeKind::Process;
            node.is_rework = false;
        }
        "ryen-decision" => {
            node.kind = NodeKind::Decision;
            node.is_rework = false;
        }
        "ryen-success" => {
            // Legacy documents used this class for any positive result. Under
            // the semantic palette, only an actual start/end terminal keeps
            // the green pill; ordinary successful actions remain processes.
            node.kind = if node.kind == NodeKind::Terminal || is_terminal_label(&node.label) {
                NodeKind::Terminal
            } else {
                NodeKind::Process
            };
            node.is_rework = false;
        }
        "ryen-rework" => {
            node.kind = NodeKind::Process;
            node.is_rework = true;
        }
        _ => return,
    }
    node.tone = tone_for_kind(node.kind);
    let (width, height) = measure_node(&node.label, node.kind);
    node.width = width;
    node.height = height;
}

fn measure_node(label: &str, kind: NodeKind) -> (f64, f64) {
    let lines = wrap_label(label, 16);
    let widest = lines
        .iter()
        .map(|line| text_width(line))
        .fold(0.0_f64, f64::max);
    let padding = if kind == NodeKind::Decision {
        58.0
    } else {
        40.0
    };
    let width = (widest + padding).clamp(132.0, 232.0);
    let mut height = (lines.len() as f64 * 21.0 + 30.0).max(NODE_HEIGHT);
    if kind == NodeKind::Decision {
        // Keep the two diagonals visually balanced as labels vary in length.
        height = height.max(width * 0.68);
    }
    (width, height)
}

fn text_width(value: &str) -> f64 {
    value
        .chars()
        .map(|character| if character.is_ascii() { 8.2 } else { 15.0 })
        .sum()
}

fn wrap_label(value: &str, max_units: usize) -> Vec<String> {
    let mut result = Vec::new();
    for explicit_line in value.lines() {
        let mut current = String::new();
        let mut units = 0;
        for character in explicit_line.chars() {
            let cost = if character.is_ascii() { 1 } else { 2 };
            if units + cost > max_units && !current.is_empty() {
                result.push(current);
                current = String::new();
                units = 0;
            }
            current.push(character);
            units += cost;
        }
        if !current.is_empty() {
            result.push(current);
        }
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

fn layout_graph(graph: &mut Graph) -> Result<(), String> {
    let components = strongly_connected_components(&graph.nodes, &graph.edges);
    let mut component_of = vec![0_usize; graph.nodes.len()];
    for (component_index, component) in components.iter().enumerate() {
        for &node in component {
            component_of[node] = component_index;
        }
    }

    let mut component_edges = HashSet::<(usize, usize)>::new();
    let mut indegree = vec![0_usize; components.len()];
    let mut outgoing = vec![Vec::<usize>::new(); components.len()];
    for edge in &graph.edges {
        let from = component_of[edge.from];
        let to = component_of[edge.to];
        if from != to && component_edges.insert((from, to)) {
            outgoing[from].push(to);
            indegree[to] += 1;
        }
    }

    let mut component_rank = vec![0_usize; components.len()];
    let mut queue = VecDeque::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            queue.push_back(index);
        }
    }
    let mut visited = 0;
    while let Some(component) = queue.pop_front() {
        visited += 1;
        let next_rank = component_rank[component] + components[component].len().max(1);
        for &next in &outgoing[component] {
            component_rank[next] = component_rank[next].max(next_rank);
            indegree[next] -= 1;
            if indegree[next] == 0 {
                queue.push_back(next);
            }
        }
    }
    if visited != components.len() {
        return Err("Unable to derive a stable flowchart layout".to_string());
    }

    for (component_index, component) in components.iter().enumerate() {
        let mut ordered = component.clone();
        ordered.sort_by_key(|&node| graph.nodes[node].source_order);
        for (local_rank, &node) in ordered.iter().enumerate() {
            graph.nodes[node].rank = component_rank[component_index] + local_rank;
        }
    }

    normalize_top_down_group_ranks(graph);

    for edge_index in 0..graph.edges.len() {
        let internal_top_down = {
            let edge = &graph.edges[edge_index];
            is_internal_top_down_edge(graph, edge)
        };
        let edge = &mut graph.edges[edge_index];
        edge.is_return = !internal_top_down
            && (edge.from == edge.to || graph.nodes[edge.to].rank <= graph.nodes[edge.from].rank);
    }

    minimize_crossings(graph);
    apply_top_down_group_orders(graph);
    assign_coordinates(graph);
    // Rework nodes are written in standard Mermaid as a branch from a decision.
    // The visual design also needs the return leg to the decision's upstream
    // node, but that leg is deliberately synthesized after layout so it does
    // not participate in rank calculation or distort the main flow.
    add_implicit_rework_returns(graph);
    Ok(())
}

// A top-down subgraph is a compound node in the outer left-to-right layout.
// Its members may be connected vertically, but those internal edges must not
// create extra outer ranks or be mistaken for return edges.
fn normalize_top_down_group_ranks(graph: &mut Graph) {
    let top_down_groups: Vec<String> = graph
        .groups
        .iter()
        .filter(|group| group.direction == Some(Direction::TopDown))
        .map(|group| group.id.clone())
        .collect();
    if top_down_groups.is_empty() {
        return;
    }

    for _ in 0..graph.nodes.len().saturating_mul(2).max(1) {
        let mut changed = false;
        for group_id in &top_down_groups {
            let members: Vec<usize> = graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node_in_group_or_child(graph, node, group_id))
                .map(|(index, _)| index)
                .collect();
            if members.is_empty() {
                continue;
            }
            let mut target_rank = members
                .iter()
                .map(|&index| graph.nodes[index].rank)
                .min()
                .unwrap_or(0);
            for edge in &graph.edges {
                if !members.contains(&edge.to) || members.contains(&edge.from) {
                    continue;
                }
                target_rank = target_rank.max(graph.nodes[edge.from].rank + 1);
            }
            for index in members {
                if graph.nodes[index].rank != target_rank {
                    graph.nodes[index].rank = target_rank;
                    changed = true;
                }
            }
        }

        for edge in &graph.edges {
            if is_internal_top_down_edge(graph, edge) || edge.from == edge.to {
                continue;
            }
            let required = graph.nodes[edge.from].rank + 1;
            if graph.nodes[edge.to].rank < required {
                graph.nodes[edge.to].rank = required;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Remove gaps introduced by collapsing a compound subgraph. This keeps
    // the wrapped LR layout compact and prevents a phantom empty column.
    let mut ranks: Vec<usize> = graph.nodes.iter().map(|node| node.rank).collect();
    ranks.sort_unstable();
    ranks.dedup();
    for node in &mut graph.nodes {
        if let Ok(rank) = ranks.binary_search(&node.rank) {
            node.rank = rank;
        }
    }

    for group_id in top_down_groups {
        let mut members: Vec<usize> = graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node_in_group_or_child(graph, node, &group_id))
            .map(|(index, _)| index)
            .collect();
        members.sort_by_key(|&index| graph.nodes[index].source_order);
        for (slot, index) in members.into_iter().enumerate() {
            graph.nodes[index].order = slot as f64;
        }
    }
}

fn apply_top_down_group_orders(graph: &mut Graph) {
    let mut ranks = graph.nodes.iter().map(|node| node.rank).collect::<Vec<_>>();
    ranks.sort_unstable();
    ranks.dedup();

    for rank in ranks {
        let rank_nodes = graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.rank == rank)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let mut seen_groups = HashSet::<String>::new();
        let mut units = Vec::<(usize, Vec<usize>)>::new();

        for index in rank_nodes {
            let group_id = top_down_group_id(graph, &graph.nodes[index]).map(str::to_owned);
            if let Some(group_id) = group_id {
                if !seen_groups.insert(group_id.clone()) {
                    continue;
                }
                let mut members = graph
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, node)| {
                        node.rank == rank
                            && top_down_group_id(graph, node) == Some(group_id.as_str())
                    })
                    .map(|(member_index, _)| member_index)
                    .collect::<Vec<_>>();
                members.sort_by_key(|&member_index| graph.nodes[member_index].source_order);
                let first_source = members
                    .iter()
                    .map(|&member_index| graph.nodes[member_index].source_order)
                    .min()
                    .unwrap_or(graph.nodes[index].source_order);
                units.push((first_source, members));
            } else {
                units.push((graph.nodes[index].source_order, vec![index]));
            }
        }

        units.sort_by_key(|(first_source, _)| *first_source);
        let mut order = 0.0;
        for (_, members) in units {
            for index in members {
                graph.nodes[index].order = order;
                order += 1.0;
            }
        }
    }
}

fn node_in_group_or_child(graph: &Graph, node: &Node, group_id: &str) -> bool {
    let mut current = node.group.as_deref();
    while let Some(candidate) = current {
        if candidate == group_id {
            return true;
        }
        current = graph
            .groups
            .iter()
            .find(|group| group.id == candidate)
            .and_then(|group| group.parent.as_deref());
    }
    false
}

fn top_down_group_id<'a>(graph: &'a Graph, node: &Node) -> Option<&'a str> {
    let mut current = node.group.as_deref();
    while let Some(candidate) = current {
        let group = graph.groups.iter().find(|group| group.id == candidate)?;
        if group.direction == Some(Direction::TopDown) {
            return Some(group.id.as_str());
        }
        current = group.parent.as_deref();
    }
    None
}

fn is_internal_top_down_edge(graph: &Graph, edge: &Edge) -> bool {
    let from_group = top_down_group_id(graph, &graph.nodes[edge.from]);
    let to_group = top_down_group_id(graph, &graph.nodes[edge.to]);
    from_group.is_some() && from_group == to_group
}

fn group_block_gap_between(graph: &Graph, left: usize, right: usize) -> f64 {
    let left_group = top_down_group_id(graph, &graph.nodes[left]);
    let right_group = top_down_group_id(graph, &graph.nodes[right]);
    if left_group != right_group && (left_group.is_some() || right_group.is_some()) {
        GROUP_BLOCK_GAP
    } else {
        0.0
    }
}

fn group_block_gap_total(graph: &Graph, nodes: &[usize]) -> f64 {
    nodes
        .windows(2)
        .map(|pair| group_block_gap_between(graph, pair[0], pair[1]))
        .sum()
}

fn strongly_connected_components(nodes: &[Node], edges: &[Edge]) -> Vec<Vec<usize>> {
    struct Tarjan<'a> {
        adjacency: Vec<Vec<usize>>,
        index: usize,
        indices: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        components: Vec<Vec<usize>>,
        _nodes: &'a [Node],
    }

    impl Tarjan<'_> {
        fn visit(&mut self, node: usize) {
            self.indices[node] = Some(self.index);
            self.lowlink[node] = self.index;
            self.index += 1;
            self.stack.push(node);
            self.on_stack[node] = true;

            for next in self.adjacency[node].clone() {
                if self.indices[next].is_none() {
                    self.visit(next);
                    self.lowlink[node] = self.lowlink[node].min(self.lowlink[next]);
                } else if self.on_stack[next] {
                    self.lowlink[node] = self.lowlink[node].min(self.indices[next].unwrap_or(0));
                }
            }

            if self.lowlink[node] == self.indices[node].unwrap_or(usize::MAX) {
                let mut component = Vec::new();
                while let Some(member) = self.stack.pop() {
                    self.on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                self.components.push(component);
            }
        }
    }

    let mut adjacency = vec![Vec::new(); nodes.len()];
    for edge in edges {
        adjacency[edge.from].push(edge.to);
    }
    let mut state = Tarjan {
        adjacency,
        index: 0,
        indices: vec![None; nodes.len()],
        lowlink: vec![0; nodes.len()],
        stack: Vec::new(),
        on_stack: vec![false; nodes.len()],
        components: Vec::new(),
        _nodes: nodes,
    };
    for node in 0..nodes.len() {
        if state.indices[node].is_none() {
            state.visit(node);
        }
    }
    state.components
}

fn minimize_crossings(graph: &mut Graph) {
    let max_rank = graph.nodes.iter().map(|node| node.rank).max().unwrap_or(0);
    for node in &mut graph.nodes {
        node.order = node.source_order as f64;
    }

    for _ in 0..4 {
        for rank in 1..=max_rank {
            let mut layer: Vec<usize> = graph
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| (node.rank == rank).then_some(index))
                .collect();
            layer.sort_by(|&left, &right| {
                barycenter(graph, left, true)
                    .partial_cmp(&barycenter(graph, right, true))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| {
                        graph.nodes[left]
                            .source_order
                            .cmp(&graph.nodes[right].source_order)
                    })
            });
            for (order, index) in layer.into_iter().enumerate() {
                graph.nodes[index].order = order as f64;
            }
        }
        for rank in (0..max_rank).rev() {
            let mut layer: Vec<usize> = graph
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| (node.rank == rank).then_some(index))
                .collect();
            layer.sort_by(|&left, &right| {
                barycenter(graph, left, false)
                    .partial_cmp(&barycenter(graph, right, false))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| {
                        graph.nodes[left]
                            .source_order
                            .cmp(&graph.nodes[right].source_order)
                    })
            });
            for (order, index) in layer.into_iter().enumerate() {
                graph.nodes[index].order = order as f64;
            }
        }
    }
}

fn barycenter(graph: &Graph, node: usize, incoming: bool) -> f64 {
    let neighbors: Vec<f64> = graph
        .edges
        .iter()
        .filter(|edge| !edge.is_return)
        .filter_map(|edge| {
            if incoming && edge.to == node {
                Some(graph.nodes[edge.from].order)
            } else if !incoming && edge.from == node {
                Some(graph.nodes[edge.to].order)
            } else {
                None
            }
        })
        .collect();
    if neighbors.is_empty() {
        graph.nodes[node].order
    } else {
        neighbors.iter().sum::<f64>() / neighbors.len() as f64
    }
}

fn assign_coordinates(graph: &mut Graph) {
    let max_rank = graph.nodes.iter().map(|node| node.rank).max().unwrap_or(0);
    let return_count = graph.edges.iter().filter(|edge| edge.is_return).count();
    let return_pad = if return_count == 0 {
        0.0
    } else {
        34.0 + return_count.min(6) as f64 * 14.0
    };
    let mut layers = vec![Vec::<usize>::new(); max_rank + 1];
    for (index, node) in graph.nodes.iter().enumerate() {
        layers[node.rank].push(index);
    }
    for layer in &mut layers {
        layer.sort_by(|&left, &right| {
            graph.nodes[left]
                .order
                .partial_cmp(&graph.nodes[right].order)
                .unwrap_or(Ordering::Equal)
        });
    }
    match graph.direction {
        Direction::LeftToRight => {
            let layer_widths: Vec<f64> = layers
                .iter()
                .map(|layer| {
                    layer
                        .iter()
                        .map(|&index| graph.nodes[index].width)
                        .fold(108.0_f64, f64::max)
                })
                .collect();
            let natural_width = layer_widths.iter().sum::<f64>()
                + RANK_GAP * layer_widths.len().saturating_sub(1) as f64
                + MARGIN * 2.0;
            if natural_width > MAX_LAYOUT_WIDTH && layers.len() > 2 {
                assign_wrapped_left_to_right(graph, &layers, &layer_widths);
                return;
            }
            let layer_heights: Vec<f64> = layers
                .iter()
                .map(|layer| {
                    layer
                        .iter()
                        .map(|&index| graph.nodes[index].height)
                        .sum::<f64>()
                        + NODE_GAP * layer.len().saturating_sub(1) as f64
                        + group_block_gap_total(graph, layer)
                })
                .collect();
            let content_height = layer_heights.iter().copied().fold(0.0_f64, f64::max);
            let mut x = MARGIN;
            for (rank, layer) in layers.iter().enumerate() {
                let max_width = layer
                    .iter()
                    .map(|&index| graph.nodes[index].width)
                    .fold(108.0_f64, f64::max);
                let mut y = MARGIN + return_pad + (content_height - layer_heights[rank]) / 2.0;
                for (position, &index) in layer.iter().enumerate() {
                    if position > 0 {
                        y += NODE_GAP + group_block_gap_between(graph, layer[position - 1], index);
                    }
                    graph.nodes[index].x = x + (max_width - graph.nodes[index].width) / 2.0;
                    graph.nodes[index].y = y;
                    y += graph.nodes[index].height;
                }
                x += max_width + RANK_GAP;
            }
        }
        Direction::TopDown => {
            let layer_widths: Vec<f64> = layers
                .iter()
                .map(|layer| {
                    layer
                        .iter()
                        .map(|&index| graph.nodes[index].width)
                        .sum::<f64>()
                        + NODE_GAP * layer.len().saturating_sub(1) as f64
                        + group_block_gap_total(graph, layer)
                })
                .collect();
            let content_width = layer_widths.iter().copied().fold(0.0_f64, f64::max);
            let mut y = MARGIN;
            for (rank, layer) in layers.iter().enumerate() {
                let max_height = layer
                    .iter()
                    .map(|&index| graph.nodes[index].height)
                    .fold(NODE_HEIGHT, f64::max);
                let mut x = MARGIN + (content_width - layer_widths[rank]) / 2.0;
                for (position, &index) in layer.iter().enumerate() {
                    if position > 0 {
                        x += NODE_GAP + group_block_gap_between(graph, layer[position - 1], index);
                    }
                    graph.nodes[index].x = x;
                    graph.nodes[index].y = y + (max_height - graph.nodes[index].height) / 2.0;
                    x += graph.nodes[index].width;
                }
                y += max_height + RANK_GAP;
            }
        }
    }
}

fn assign_wrapped_left_to_right(graph: &mut Graph, layers: &[Vec<usize>], layer_widths: &[f64]) {
    // Rework nodes are positioned in a dedicated lane. Excluding them from
    // the main layer prevents an empty slot from pushing wrapped rows apart.
    let layer_heights: Vec<f64> = layers
        .iter()
        .map(|layer| main_layer_height(graph, layer))
        .collect();
    let branch_lane = (0..graph.nodes.len())
        .map(|index| is_branch_lane_node(graph, index))
        .collect::<Vec<_>>();

    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0.0;
    for (rank, width) in layer_widths.iter().copied().enumerate() {
        let next_width = if current.is_empty() {
            width
        } else {
            current_width + RANK_GAP + width
        };
        if !current.is_empty() && next_width + MARGIN * 2.0 > MAX_LAYOUT_WIDTH {
            rows.push(current);
            current = Vec::new();
            current_width = 0.0;
        }
        current.push(rank);
        current_width = if current_width == 0.0 {
            width
        } else {
            current_width + RANK_GAP + width
        };
    }
    if !current.is_empty() {
        rows.push(current);
    }

    let mut row_of_rank = vec![0_usize; layers.len()];
    for (row_index, row) in rows.iter().enumerate() {
        for &rank in row {
            row_of_rank[rank] = row_index;
        }
    }

    let mut row_heights: Vec<f64> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&rank| layer_heights[rank])
                .fold(0.0_f64, f64::max)
        })
        .collect();
    let mut lane_heights = vec![0.0_f64; rows.len()];
    let mut branch_groups = HashMap::<usize, Vec<usize>>::new();
    for index in 0..graph.nodes.len() {
        if let Some(source) = branch_source(graph, index) {
            branch_groups.entry(source).or_default().push(index);
        }
    }
    let mut branch_slot = vec![0_usize; graph.nodes.len()];
    let mut branch_stack_height = vec![0.0_f64; graph.nodes.len()];
    for branch_nodes in branch_groups.values() {
        let stack_height = branch_nodes
            .iter()
            .map(|&index| graph.nodes[index].height)
            .sum::<f64>()
            + NODE_GAP * branch_nodes.len().saturating_sub(1) as f64;
        for (slot, &index) in branch_nodes.iter().enumerate() {
            branch_slot[index] = slot;
            branch_stack_height[index] = stack_height;
        }
    }
    for row_index in 0..rows.len() {
        let mut by_source = HashMap::<usize, Vec<usize>>::new();
        for index in 0..graph.nodes.len() {
            let Some(source) = branch_source(graph, index) else {
                continue;
            };
            if row_of_rank[graph.nodes[source].rank] == row_index {
                by_source
                    .entry(graph.nodes[source].rank)
                    .or_default()
                    .push(index);
            }
        }
        lane_heights[row_index] = by_source
            .values()
            .map(|branch_nodes| {
                branch_nodes
                    .iter()
                    .map(|&index| graph.nodes[index].height)
                    .sum::<f64>()
                    + NODE_GAP * branch_nodes.len().saturating_sub(1) as f64
                    + BRANCH_GAP
            })
            .fold(0.0_f64, f64::max);
        row_heights[row_index] += lane_heights[row_index];
    }

    let widest_row = rows
        .iter()
        .map(|row| {
            row.iter().map(|&rank| layer_widths[rank]).sum::<f64>()
                + RANK_GAP * row.len().saturating_sub(1) as f64
        })
        .fold(0.0_f64, f64::max);

    let mut row_main_y = vec![0.0_f64; rows.len()];
    let mut row_y = MARGIN;
    for (row_index, row_height) in row_heights.iter().copied().enumerate() {
        let main_height = row_height - lane_heights[row_index];
        let main_top = if row_index % 2 == 0 {
            row_y + lane_heights[row_index]
        } else {
            row_y
        };
        row_main_y[row_index] = main_top + main_height / 2.0;
        row_y += row_height + ROW_GAP;
    }
    row_y = MARGIN;
    for (row_index, row) in rows.iter().enumerate() {
        let row_height = row_heights[row_index];
        let row_width = row.iter().map(|&rank| layer_widths[rank]).sum::<f64>()
            + RANK_GAP * row.len().saturating_sub(1) as f64;
        let row_offset = if row_index % 2 == 0 {
            0.0
        } else {
            widest_row - row_width
        };
        let mut x = MARGIN + row_offset;
        let ranks: Vec<usize> = if row_index % 2 == 0 {
            row.clone()
        } else {
            row.iter().rev().copied().collect()
        };

        for rank in ranks {
            let layer_height = layer_heights[rank];
            let main_height = row_height - lane_heights[row_index];
            let main_top = if row_index % 2 == 0 {
                row_y + lane_heights[row_index]
            } else {
                row_y
            };
            let mut y = main_top + (main_height - layer_height) / 2.0;
            let main_nodes = layers[rank]
                .iter()
                .copied()
                .filter(|&index| !branch_lane[index])
                .collect::<Vec<_>>();
            for (position, index) in main_nodes.iter().copied().enumerate() {
                if position > 0 {
                    y += NODE_GAP + group_block_gap_between(graph, main_nodes[position - 1], index);
                }
                let node = &mut graph.nodes[index];
                node.x = x + (layer_widths[rank] - node.width) / 2.0;
                node.y = y;
                y += node.height;
            }
            x += layer_widths[rank] + RANK_GAP;
        }
        row_y += row_height + ROW_GAP;
    }

    // Rework nodes use a semantic side lane next to their incoming decision.
    // The lane alternates with the wrapped row, so it works for arbitrary
    // long graphs rather than depending on particular node identifiers.
    let rework_sources = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, _)| branch_source(graph, index))
        .collect::<Vec<_>>();
    let rework_source_geometry = rework_sources
        .iter()
        .map(|source| {
            source.map(|index| {
                (
                    graph.nodes[index].rank,
                    graph.nodes[index].x + graph.nodes[index].width / 2.0,
                )
            })
        })
        .collect::<Vec<_>>();
    for (index, node) in graph.nodes.iter_mut().enumerate() {
        if !is_rework_node(node) {
            continue;
        }
        let Some((source_rank, source_center_x)) = rework_source_geometry[index] else {
            continue;
        };
        let row = row_of_rank[source_rank];
        node.x = source_center_x - node.width / 2.0;
        let main_height = row_heights[row] - lane_heights[row];
        let main_top = row_main_y[row] - main_height / 2.0;
        let stack_height = branch_stack_height[index];
        let stack_y = if row % 2 == 0 {
            main_top - BRANCH_GAP - stack_height
                + (lane_heights[row] - BRANCH_GAP - stack_height).max(0.0) / 2.0
        } else {
            main_top
                + main_height
                + BRANCH_GAP
                + (lane_heights[row] - BRANCH_GAP - stack_height).max(0.0) / 2.0
        };
        let branch_offset = branch_slot[index] as f64 * (node.height + NODE_GAP);
        node.y = if row % 2 == 0 {
            stack_y + branch_offset
        } else {
            stack_y + branch_offset
        };
    }
}

fn main_layer_height(graph: &Graph, layer: &[usize]) -> f64 {
    let main_nodes = layer
        .iter()
        .copied()
        .filter(|&index| !is_branch_lane_node(graph, index))
        .collect::<Vec<_>>();
    main_nodes
        .iter()
        .map(|&index| graph.nodes[index].height)
        .sum::<f64>()
        + NODE_GAP * main_nodes.len().saturating_sub(1) as f64
        + group_block_gap_total(graph, &main_nodes)
}

fn branch_source(graph: &Graph, index: usize) -> Option<usize> {
    if !is_rework_node(&graph.nodes[index]) {
        return None;
    }
    graph
        .edges
        .iter()
        .find(|edge| edge.to == index && graph.nodes[edge.from].kind == NodeKind::Decision)
        .map(|edge| edge.from)
}

fn is_branch_lane_node(graph: &Graph, index: usize) -> bool {
    branch_source(graph, index).is_some()
}

fn is_rework_node(node: &Node) -> bool {
    node.kind == NodeKind::Process && node.is_rework
}

fn add_implicit_rework_returns(graph: &mut Graph) {
    let mut additions = Vec::<(usize, usize)>::new();
    for rework_index in 0..graph.nodes.len() {
        if !is_rework_node(&graph.nodes[rework_index]) {
            continue;
        }
        let Some(decision_index) = graph
            .edges
            .iter()
            .filter(|edge| {
                edge.to == rework_index && graph.nodes[edge.from].kind == NodeKind::Decision
            })
            .min_by_key(|edge| edge.source_order)
            .map(|edge| edge.from)
        else {
            continue;
        };
        let return_target = graph.nodes[rework_index].return_target.clone();
        let hinted_upstream = return_target.as_deref().and_then(|hint| {
            graph
                .nodes
                .iter()
                .enumerate()
                .find(|(index, node)| {
                    *index != rework_index && (node.id == hint || node.label == hint)
                })
                .map(|(index, _)| index)
        });
        let upstream_index = hinted_upstream.or_else(|| {
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to == decision_index
                        && edge.from != rework_index
                        && !is_rework_node(&graph.nodes[edge.from])
                })
                .min_by_key(|edge| edge.source_order)
                .map(|edge| edge.from)
        });
        let Some(upstream_index) = upstream_index else {
            continue;
        };
        if upstream_index == rework_index
            || graph
                .edges
                .iter()
                .any(|edge| edge.from == rework_index && edge.to == upstream_index)
        {
            continue;
        }
        additions.push((rework_index, upstream_index));
    }

    let next_source_order = graph
        .edges
        .iter()
        .map(|edge| edge.source_order)
        .max()
        .map_or(0, |order| order + 1);
    for (offset, (from, to)) in additions.into_iter().enumerate() {
        graph.edges.push(Edge {
            from,
            to,
            label: String::new(),
            dashed: true,
            arrow: true,
            is_return: true,
            source_order: next_source_order + offset,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnPort {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
struct ReturnPlan {
    start: ReturnPort,
    end: ReturnPort,
}

#[derive(Clone, Copy, Debug)]
struct ReturnPath {
    start_x: f64,
    start_y: f64,
    control1_x: f64,
    control1_y: f64,
    control2_x: f64,
    control2_y: f64,
    end_x: f64,
    end_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RoutePoint {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForwardBypassSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug)]
struct ForwardBypassRoute {
    points: Vec<RoutePoint>,
    label_x: f64,
    label_y: f64,
    side: ForwardBypassSide,
}

fn return_plan_for(graph: &Graph, edge: &Edge, channel: usize) -> ReturnPlan {
    if edge.from == edge.to {
        return ReturnPlan {
            start: ReturnPort::Right,
            end: ReturnPort::Right,
        };
    }
    let ports = [
        // Deterministic lateral tie-breaker: the right side usually keeps
        // the return lane away from the document's left margin.
        ReturnPort::Right,
        ReturnPort::Left,
        ReturnPort::Top,
        ReturnPort::Bottom,
    ];
    ports
        .into_iter()
        .flat_map(|start| ports.into_iter().map(move |end| ReturnPlan { start, end }))
        .min_by(|left, right| {
            return_plan_score(graph, edge, *left, channel)
                .partial_cmp(&return_plan_score(graph, edge, *right, channel))
                .unwrap_or(Ordering::Equal)
        })
        .unwrap_or(ReturnPlan {
            start: ReturnPort::Left,
            end: ReturnPort::Right,
        })
}

fn return_plan_score(graph: &Graph, edge: &Edge, plan: ReturnPlan, channel: usize) -> f64 {
    let from = &graph.nodes[edge.from];
    let to = &graph.nodes[edge.to];
    let from_center_x = from.x + from.width / 2.0;
    let from_center_y = from.y + from.height / 2.0;
    let to_center_x = to.x + to.width / 2.0;
    let to_center_y = to.y + to.height / 2.0;
    let horizontal_delta = (from_center_x - to_center_x).abs();
    let vertical_delta = (from_center_y - to_center_y).abs();
    let path = return_path_for(graph, edge, plan, channel);
    let travel = (path.start_x - path.control1_x).abs()
        + (path.start_y - path.control1_y).abs()
        + (path.control1_x - path.control2_x).abs()
        + (path.control1_y - path.control2_y).abs()
        + (path.control2_x - path.end_x).abs()
        + (path.control2_y - path.end_y).abs();

    let desired_start = if horizontal_delta > vertical_delta * 0.65 {
        if to_center_x < from_center_x {
            ReturnPort::Left
        } else {
            ReturnPort::Right
        }
    } else if to_center_y < from_center_y {
        ReturnPort::Top
    } else {
        ReturnPort::Bottom
    };
    let desired_end = if vertical_delta > horizontal_delta * 0.65 {
        if from_center_y < to_center_y {
            ReturnPort::Top
        } else {
            ReturnPort::Bottom
        }
    } else if from_center_x < to_center_x {
        ReturnPort::Left
    } else {
        ReturnPort::Right
    };
    let port_penalty = (plan.start != desired_start) as u8 as f64 * 150.0
        + (plan.end != desired_end) as u8 as f64 * 120.0;
    let obstacle_penalty = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != edge.from && *index != edge.to)
        .filter(|(_, node)| return_path_hits_node(&path, node))
        .count() as f64
        * 1400.0;
    let group_penalty = graph
        .groups
        .iter()
        .filter(|group| !group_contains_endpoint(graph, group, edge))
        .filter(|group| return_path_hits_group(graph, group, &path))
        .count() as f64
        * 2200.0;
    travel + port_penalty + obstacle_penalty + group_penalty + channel.min(7) as f64 * 12.0
}

fn return_path_for(graph: &Graph, edge: &Edge, plan: ReturnPlan, channel: usize) -> ReturnPath {
    let from = &graph.nodes[edge.from];
    let to = &graph.nodes[edge.to];
    let (start_x, start_y) = return_port_point(from, plan.start);
    let (end_x, end_y) = return_port_point(to, plan.end);
    let (start_dx, start_dy) = return_port_normal(plan.start);
    let (end_dx, end_dy) = return_port_normal(plan.end);
    let mut gap = return_curve_handle_length(start_x, start_y, end_x, end_y, plan, channel);
    let lane_offset = 34.0 + channel.min(7) as f64 * 18.0;
    if plan.start == plan.end {
        gap = match plan.start {
            ReturnPort::Left => {
                let outer = graph
                    .nodes
                    .iter()
                    .map(|node| node.x)
                    .fold(f64::MAX, f64::min)
                    - lane_offset;
                gap.max(start_x.max(end_x) - outer)
            }
            ReturnPort::Right => {
                let outer = graph
                    .nodes
                    .iter()
                    .map(|node| node.x + node.width)
                    .fold(0.0_f64, f64::max)
                    + lane_offset;
                gap.max(outer - start_x.min(end_x))
            }
            ReturnPort::Top => {
                let outer = graph
                    .nodes
                    .iter()
                    .map(|node| node.y)
                    .fold(f64::MAX, f64::min)
                    - lane_offset;
                gap.max(start_y.max(end_y) - outer)
            }
            ReturnPort::Bottom => {
                let outer = graph
                    .nodes
                    .iter()
                    .map(|node| node.y + node.height)
                    .fold(0.0_f64, f64::max)
                    + lane_offset;
                gap.max(outer - start_y.min(end_y))
            }
        };
    }
    ReturnPath {
        start_x,
        start_y,
        control1_x: start_x + start_dx * gap,
        control1_y: start_y + start_dy * gap,
        control2_x: end_x + end_dx * gap,
        control2_y: end_y + end_dy * gap,
        end_x,
        end_y,
    }
}

fn return_curve_handle_length(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    plan: ReturnPlan,
    channel: usize,
) -> f64 {
    let base = 32.0 + channel.min(7) as f64 * 14.0;
    let horizontal_start = matches!(plan.start, ReturnPort::Left | ReturnPort::Right);
    let horizontal_end = matches!(plan.end, ReturnPort::Left | ReturnPort::Right);
    let span = (start_x - end_x).abs().max((start_y - end_y).abs());

    // Perpendicular ports need a larger, rounded sweep when the endpoints
    // are far apart. With a fixed 32px handle, long returns collapse into a
    // nearly straight diagonal (the old behaviour of curve ②). The 0.28
    // factor gives the long arc more body while the cap keeps very large
    // diagrams from producing an exaggerated balloon. Short perpendicular
    // returns stay at the compact baseline, preserving the look of curve ③.
    if horizontal_start != horizontal_end {
        let long_span = ((span - 132.0).max(0.0) * 2.3).min(128.0);
        return base + long_span;
    }

    // Opposing ports benefit from a small amount of extra tension. Same-side
    // ports, such as curve ③, already form the desired compact circular arc
    // with the channel spacing alone, so keep their handle unchanged.
    if plan.start != plan.end {
        return base + (span * 0.12).min(72.0);
    }

    base
}

fn return_port_point(node: &Node, port: ReturnPort) -> (f64, f64) {
    match port {
        ReturnPort::Left => (node.x, node.y + node.height / 2.0),
        ReturnPort::Right => (node.x + node.width, node.y + node.height / 2.0),
        ReturnPort::Top => (node.x + node.width / 2.0, node.y),
        ReturnPort::Bottom => (node.x + node.width / 2.0, node.y + node.height),
    }
}

fn return_port_normal(port: ReturnPort) -> (f64, f64) {
    match port {
        ReturnPort::Left => (-1.0, 0.0),
        ReturnPort::Right => (1.0, 0.0),
        ReturnPort::Top => (0.0, -1.0),
        ReturnPort::Bottom => (0.0, 1.0),
    }
}

fn return_path_hits_node(path: &ReturnPath, node: &Node) -> bool {
    return_path_hits_rect(
        path,
        node.x - 10.0,
        node.y - 10.0,
        node.x + node.width + 10.0,
        node.y + node.height + 10.0,
    )
}

fn return_path_hits_group(graph: &Graph, group: &Group, path: &ReturnPath) -> bool {
    let members: Vec<&Node> = graph
        .nodes
        .iter()
        .filter(|node| node.group.as_deref() == Some(group.id.as_str()))
        .collect();
    if members.is_empty() {
        return false;
    }
    let min_x = members.iter().map(|node| node.x).fold(f64::MAX, f64::min) - 24.0;
    let min_y = members.iter().map(|node| node.y).fold(f64::MAX, f64::min) - 34.0;
    let max_x = members
        .iter()
        .map(|node| node.x + node.width)
        .fold(0.0_f64, f64::max)
        + 24.0;
    let max_y = members
        .iter()
        .map(|node| node.y + node.height)
        .fold(0.0_f64, f64::max)
        + 24.0;
    return_path_hits_rect(path, min_x - 8.0, min_y - 8.0, max_x + 8.0, max_y + 8.0)
}

fn group_contains_endpoint(graph: &Graph, group: &Group, edge: &Edge) -> bool {
    graph.nodes.iter().enumerate().any(|(index, node)| {
        node.group.as_deref() == Some(group.id.as_str()) && (index == edge.from || index == edge.to)
    })
}

fn return_path_hits_rect(
    path: &ReturnPath,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> bool {
    for step in 0..=48 {
        let t = step as f64 / 48.0;
        let inverse = 1.0 - t;
        let x = inverse.powi(3) * path.start_x
            + 3.0 * inverse.powi(2) * t * path.control1_x
            + 3.0 * inverse * t.powi(2) * path.control2_x
            + t.powi(3) * path.end_x;
        let y = inverse.powi(3) * path.start_y
            + 3.0 * inverse.powi(2) * t * path.control1_y
            + 3.0 * inverse * t.powi(2) * path.control2_y
            + t.powi(3) * path.end_y;
        if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
            return true;
        }
    }
    false
}

fn normal_edge_curve(graph: &Graph, edge: &Edge) -> ReturnPath {
    let from = &graph.nodes[edge.from];
    let to = &graph.nodes[edge.to];
    match graph.direction {
        Direction::LeftToRight => {
            let from_center_x = from.x + from.width / 2.0;
            let to_center_x = to.x + to.width / 2.0;
            let same_column =
                (from_center_x - to_center_x).abs() <= (from.width.min(to.width) * 0.35).max(18.0);
            if same_column {
                let going_up = to.y + to.height / 2.0 < from.y + from.height / 2.0;
                let start_x = from_center_x;
                let start_y = if going_up {
                    from.y
                } else {
                    from.y + from.height
                };
                let end_x = to_center_x;
                let end_y = if going_up { to.y + to.height } else { to.y };
                let bend_y = if going_up {
                    start_y - 28.0
                } else {
                    start_y + 28.0
                };
                ReturnPath {
                    start_x,
                    start_y,
                    control1_x: start_x,
                    control1_y: bend_y,
                    control2_x: end_x,
                    control2_y: bend_y,
                    end_x,
                    end_y,
                }
            } else {
                let going_left = to_center_x < from_center_x;
                let start_x = if going_left {
                    from.x
                } else {
                    from.x + from.width
                };
                let start_y = from.y + from.height / 2.0;
                let end_x = if going_left { to.x + to.width } else { to.x };
                let end_y = to.y + to.height / 2.0;
                let middle_x = (start_x + end_x) / 2.0;
                ReturnPath {
                    start_x,
                    start_y,
                    control1_x: middle_x,
                    control1_y: start_y,
                    control2_x: middle_x,
                    control2_y: end_y,
                    end_x,
                    end_y,
                }
            }
        }
        Direction::TopDown => {
            let start_x = from.x + from.width / 2.0;
            let start_y = from.y + from.height;
            let end_x = to.x + to.width / 2.0;
            let end_y = to.y;
            let middle_y = (start_y + end_y) / 2.0;
            ReturnPath {
                start_x,
                start_y,
                control1_x: start_x,
                control1_y: middle_y,
                control2_x: end_x,
                control2_y: middle_y,
                end_x,
                end_y,
            }
        }
    }
}

fn forward_bypass_route(graph: &Graph, edge: &Edge) -> Option<ForwardBypassRoute> {
    if edge.is_return || edge.from == edge.to {
        return None;
    }
    let direct = normal_edge_curve(graph, edge);
    let direct_hits_obstacle = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != edge.from && *index != edge.to)
        .any(|(_, node)| return_path_hits_node(&direct, node));
    if !direct_hits_obstacle {
        return None;
    }

    let candidates = match edge_routing_direction(graph, edge) {
        Direction::TopDown => vec![
            top_down_bypass_candidate(graph, edge, ForwardBypassSide::Left),
            top_down_bypass_candidate(graph, edge, ForwardBypassSide::Right),
        ],
        Direction::LeftToRight => vec![
            left_to_right_bypass_candidate(graph, edge, ForwardBypassSide::Top),
            left_to_right_bypass_candidate(graph, edge, ForwardBypassSide::Bottom),
        ],
    };

    candidates
        .into_iter()
        .map(|route| {
            let score = forward_bypass_score(graph, edge, &route);
            (route, score)
        })
        .filter(|(_, score)| *score < 10_000.0)
        .min_by(|left, right| left.1.partial_cmp(&right.1).unwrap_or(Ordering::Equal))
        .map(|(route, _)| route)
}

fn top_down_bypass_candidate(
    graph: &Graph,
    edge: &Edge,
    side: ForwardBypassSide,
) -> ForwardBypassRoute {
    let from = &graph.nodes[edge.from];
    let to = &graph.nodes[edge.to];
    let start = match side {
        ForwardBypassSide::Left => RoutePoint {
            x: from.x,
            y: from.y + from.height / 2.0,
        },
        ForwardBypassSide::Right => RoutePoint {
            x: from.x + from.width,
            y: from.y + from.height / 2.0,
        },
        _ => unreachable!("top-down bypass uses a horizontal side"),
    };
    let (min_x, max_x) = horizontal_obstacle_span(graph, edge, start.y, to.y + to.height);
    let lane_clearance = if common_top_down_group_id(graph, edge).is_some() {
        18.0
    } else {
        34.0
    };
    let lane_x = match side {
        ForwardBypassSide::Left => min_x - lane_clearance,
        ForwardBypassSide::Right => max_x + lane_clearance,
        _ => unreachable!("top-down bypass uses a horizontal side"),
    };
    let end = match side {
        ForwardBypassSide::Left => RoutePoint {
            x: to.x,
            y: to.y + to.height / 2.0,
        },
        ForwardBypassSide::Right => RoutePoint {
            x: to.x + to.width,
            y: to.y + to.height / 2.0,
        },
        _ => unreachable!("top-down bypass uses a horizontal side"),
    };
    ForwardBypassRoute {
        points: vec![
            start,
            RoutePoint {
                x: lane_x,
                y: start.y,
            },
            RoutePoint {
                x: lane_x,
                y: end.y,
            },
            end,
        ],
        label_x: start.x + (lane_x - start.x) * 0.42,
        label_y: start.y - 13.0,
        side,
    }
}

fn left_to_right_bypass_candidate(
    graph: &Graph,
    edge: &Edge,
    side: ForwardBypassSide,
) -> ForwardBypassRoute {
    let from = &graph.nodes[edge.from];
    let to = &graph.nodes[edge.to];
    let start_center_x = from.x + from.width / 2.0;
    let start = match side {
        ForwardBypassSide::Top => RoutePoint {
            x: start_center_x,
            y: from.y,
        },
        ForwardBypassSide::Bottom => RoutePoint {
            x: start_center_x,
            y: from.y + from.height,
        },
        _ => unreachable!("left-to-right bypass uses a vertical side"),
    };
    let end_center_x = to.x + to.width / 2.0;
    let (min_y, max_y) = vertical_obstacle_span(graph, edge, start.x, end_center_x);
    let lane_y = match side {
        ForwardBypassSide::Top => min_y - 34.0,
        ForwardBypassSide::Bottom => max_y + 34.0,
        _ => unreachable!("left-to-right bypass uses a vertical side"),
    };
    let end = match side {
        ForwardBypassSide::Top => RoutePoint {
            x: end_center_x,
            y: to.y,
        },
        ForwardBypassSide::Bottom => RoutePoint {
            x: end_center_x,
            y: to.y + to.height,
        },
        _ => unreachable!("left-to-right bypass uses a vertical side"),
    };
    ForwardBypassRoute {
        points: vec![
            start,
            RoutePoint {
                x: start.x,
                y: lane_y,
            },
            RoutePoint {
                x: end.x,
                y: lane_y,
            },
            end,
        ],
        label_x: start.x + 13.0,
        label_y: start.y + (lane_y - start.y) * 0.42,
        side,
    }
}

fn edge_routing_direction(graph: &Graph, edge: &Edge) -> Direction {
    if is_internal_top_down_edge(graph, edge) {
        Direction::TopDown
    } else {
        graph.direction
    }
}

fn horizontal_obstacle_span(graph: &Graph, edge: &Edge, start_y: f64, end_y: f64) -> (f64, f64) {
    let min_y = start_y.min(end_y) - 12.0;
    let max_y = start_y.max(end_y) + 12.0;
    let mut min_x = graph.nodes[edge.from].x.min(graph.nodes[edge.to].x);
    let mut max_x = (graph.nodes[edge.from].x + graph.nodes[edge.from].width)
        .max(graph.nodes[edge.to].x + graph.nodes[edge.to].width);
    let group_scope = common_top_down_group_id(graph, edge);
    for node in &graph.nodes {
        if group_scope.is_some_and(|group_id| top_down_group_id(graph, node) != Some(group_id)) {
            continue;
        }
        if node.y <= max_y && node.y + node.height >= min_y {
            min_x = min_x.min(node.x);
            max_x = max_x.max(node.x + node.width);
        }
    }
    if group_scope.is_none() {
        for group in &graph.groups {
            if let Some((group_min_x, group_min_y, group_max_x, group_max_y)) =
                group_bounds(graph, group)
                && group_min_y <= max_y
                && group_max_y >= min_y
            {
                min_x = min_x.min(group_min_x);
                max_x = max_x.max(group_max_x);
            }
        }
    }
    (min_x, max_x)
}

fn common_top_down_group_id<'a>(graph: &'a Graph, edge: &Edge) -> Option<&'a str> {
    let from_group = top_down_group_id(graph, &graph.nodes[edge.from]);
    let to_group = top_down_group_id(graph, &graph.nodes[edge.to]);
    (from_group.is_some() && from_group == to_group).then_some(from_group.unwrap_or_default())
}

fn vertical_obstacle_span(graph: &Graph, edge: &Edge, start_x: f64, end_x: f64) -> (f64, f64) {
    let min_x = start_x.min(end_x) - 12.0;
    let max_x = start_x.max(end_x) + 12.0;
    let mut min_y = graph.nodes[edge.from].y.min(graph.nodes[edge.to].y);
    let mut max_y = (graph.nodes[edge.from].y + graph.nodes[edge.from].height)
        .max(graph.nodes[edge.to].y + graph.nodes[edge.to].height);
    for node in &graph.nodes {
        if node.x <= max_x && node.x + node.width >= min_x {
            min_y = min_y.min(node.y);
            max_y = max_y.max(node.y + node.height);
        }
    }
    for group in &graph.groups {
        if let Some((group_min_x, group_min_y, group_max_x, group_max_y)) =
            group_bounds(graph, group)
            && group_min_x <= max_x
            && group_max_x >= min_x
        {
            min_y = min_y.min(group_min_y);
            max_y = max_y.max(group_max_y);
        }
    }
    (min_y, max_y)
}

fn group_bounds(graph: &Graph, group: &Group) -> Option<(f64, f64, f64, f64)> {
    let members = graph
        .nodes
        .iter()
        .filter(|node| node.group.as_deref() == Some(group.id.as_str()))
        .collect::<Vec<_>>();
    if members.is_empty() {
        return None;
    }
    Some((
        members.iter().map(|node| node.x).fold(f64::MAX, f64::min) - 24.0,
        members.iter().map(|node| node.y).fold(f64::MAX, f64::min) - 34.0,
        members
            .iter()
            .map(|node| node.x + node.width)
            .fold(0.0_f64, f64::max)
            + 24.0,
        members
            .iter()
            .map(|node| node.y + node.height)
            .fold(0.0_f64, f64::max)
            + 24.0,
    ))
}

fn forward_bypass_score(graph: &Graph, edge: &Edge, route: &ForwardBypassRoute) -> f64 {
    let node_hits = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != edge.from && *index != edge.to)
        .filter(|(_, node)| {
            polyline_hits_rect(
                &route.points,
                node.x - 10.0,
                node.y - 10.0,
                node.x + node.width + 10.0,
                node.y + node.height + 10.0,
            )
        })
        .count() as f64;
    let group_hits = graph
        .groups
        .iter()
        .filter(|group| !group_contains_endpoint(graph, group, edge))
        .filter(|group| {
            group_bounds(graph, group).is_some_and(|(min_x, min_y, max_x, max_y)| {
                polyline_hits_rect(&route.points, min_x, min_y, max_x, max_y)
            })
        })
        .count() as f64;
    let travel = route
        .points
        .windows(2)
        .map(|pair| (pair[0].x - pair[1].x).abs() + (pair[0].y - pair[1].y).abs())
        .sum::<f64>();
    let from = &graph.nodes[edge.from];
    let to = &graph.nodes[edge.to];
    let target_affinity = match route.side {
        ForwardBypassSide::Left if to.x + to.width / 2.0 > from.x + from.width / 2.0 => 180.0,
        ForwardBypassSide::Right if to.x + to.width / 2.0 < from.x + from.width / 2.0 => 180.0,
        ForwardBypassSide::Top if to.y + to.height / 2.0 > from.y + from.height / 2.0 => 180.0,
        ForwardBypassSide::Bottom if to.y + to.height / 2.0 < from.y + from.height / 2.0 => 180.0,
        _ => 0.0,
    };
    travel
        + (route.points.len().saturating_sub(2) as f64 * 18.0)
        + target_affinity
        + node_hits * 100_000.0
        + group_hits * 120_000.0
}

fn polyline_hits_rect(
    points: &[RoutePoint],
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> bool {
    points.windows(2).any(|pair| {
        let segment_min_x = pair[0].x.min(pair[1].x);
        let segment_max_x = pair[0].x.max(pair[1].x);
        let segment_min_y = pair[0].y.min(pair[1].y);
        let segment_max_y = pair[0].y.max(pair[1].y);
        segment_max_x >= min_x
            && segment_min_x <= max_x
            && segment_max_y >= min_y
            && segment_min_y <= max_y
    })
}

fn rounded_polyline_path(points: &[RoutePoint], radius: f64) -> String {
    if points.len() < 2 {
        return String::new();
    }
    let mut path = format!("M {:.1} {:.1}", points[0].x, points[0].y);
    for index in 1..points.len() - 1 {
        let previous = points[index - 1];
        let corner = points[index];
        let next = points[index + 1];
        let incoming_length = (corner.x - previous.x).abs() + (corner.y - previous.y).abs();
        let outgoing_length = (next.x - corner.x).abs() + (next.y - corner.y).abs();
        let corner_radius = radius.min(incoming_length / 2.0).min(outgoing_length / 2.0);
        let before = move_toward(corner, previous, corner_radius);
        let after = move_toward(corner, next, corner_radius);
        let _ = write!(
            path,
            " L {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}",
            before.x, before.y, corner.x, corner.y, after.x, after.y
        );
    }
    let last = points[points.len() - 1];
    let _ = write!(path, " L {:.1} {:.1}", last.x, last.y);
    path
}

fn move_toward(from: RoutePoint, to: RoutePoint, distance: f64) -> RoutePoint {
    let delta_x = to.x - from.x;
    let delta_y = to.y - from.y;
    let length = delta_x.abs() + delta_y.abs();
    if length <= f64::EPSILON {
        return from;
    }
    RoutePoint {
        x: from.x + delta_x / length * distance,
        y: from.y + delta_y / length * distance,
    }
}

fn render_svg(graph: &Graph, source: &str) -> String {
    let hash = fnv1a(source.as_bytes());
    let prefix = format!("rym-{hash:08x}");
    let node_right = graph
        .nodes
        .iter()
        .map(|node| node.x + node.width)
        .fold(0.0_f64, f64::max);
    let node_bottom = graph
        .nodes
        .iter()
        .map(|node| node.y + node.height)
        .fold(0.0_f64, f64::max);
    let mut min_x = 0.0_f64;
    let mut min_y = 0.0_f64;
    let mut max_x = node_right + MARGIN;
    let mut max_y = node_bottom + MARGIN;
    let mut extent_channel = 0_usize;
    for edge in &graph.edges {
        let edge_channel = extent_channel;
        if edge.is_return {
            extent_channel += 1;
        }
        if edge.from == edge.to {
            let node = &graph.nodes[edge.from];
            max_x = max_x.max(node.x + node.width + 70.0);
            continue;
        }
        if !edge.is_return {
            continue;
        }
        let plan = return_plan_for(graph, edge, edge_channel);
        let path = return_path_for(graph, edge, plan, edge_channel);
        let path_min_x = path
            .start_x
            .min(path.control1_x)
            .min(path.control2_x)
            .min(path.end_x);
        let path_min_y = path
            .start_y
            .min(path.control1_y)
            .min(path.control2_y)
            .min(path.end_y);
        let path_max_x = path
            .start_x
            .max(path.control1_x)
            .max(path.control2_x)
            .max(path.end_x);
        let path_max_y = path
            .start_y
            .max(path.control1_y)
            .max(path.control2_y)
            .max(path.end_y);
        min_x = min_x.min(path_min_x - 28.0);
        min_y = min_y.min(path_min_y - 28.0);
        max_x = max_x.max(path_max_x + 28.0);
        max_y = max_y.max(path_max_y + 28.0);
    }
    let width = max_x - min_x;
    let height = max_y - min_y;
    let mut svg = String::with_capacity(graph.nodes.len() * 420 + graph.edges.len() * 260 + 2400);
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{min_x:.1} {min_y:.1} {width:.1} {height:.1}" role="img" aria-label="Mermaid flowchart" data-ryen-renderer="0.1.0" preserveAspectRatio="xMidYMid meet">"#
    );
    let _ = write!(
        svg,
        r##"<defs>
<filter id="{prefix}-purple-glow" x="-45%" y="-55%" width="190%" height="210%"><feDropShadow dx="0" dy="0" stdDeviation="5" flood-color="#a566ff" flood-opacity=".52"/></filter>
<filter id="{prefix}-orange-glow" x="-45%" y="-55%" width="190%" height="210%"><feDropShadow dx="0" dy="0" stdDeviation="5" flood-color="#ff741c" flood-opacity=".48"/></filter>
<filter id="{prefix}-green-glow" x="-45%" y="-55%" width="190%" height="210%"><feDropShadow dx="0" dy="0" stdDeviation="5" flood-color="#35d483" flood-opacity=".48"/></filter>
<filter id="{prefix}-edge-glow" x="-35%" y="-45%" width="170%" height="190%"><feDropShadow dx="0" dy="0" stdDeviation="3" flood-color="#a566ff" flood-opacity=".42"/></filter>
<marker id="{prefix}-arrow" markerWidth="13" markerHeight="13" refX="11.5" refY="6.5" orient="auto" markerUnits="userSpaceOnUse"><path d="M1.1 1.2 L11.5 6.5 L1.1 11.8 Z" fill="#a566ff"/></marker>
<style>
.rym-edge{{fill:none;stroke:#a566ff;stroke-width:3.4;stroke-linecap:round;stroke-linejoin:round}}
.rym-return{{stroke:#b17cff;stroke-dasharray:8 8;stroke-width:2.5;opacity:.92}}
.rym-edge-label{{font:320 15px 'consolaslxgw','Consolas NF + LXGW WenKai Mono','LXGW WenKai Mono','Microsoft YaHei',sans-serif;fill:#fff;stroke:#21172b;stroke-width:4px;paint-order:stroke;text-anchor:middle;dominant-baseline:middle;letter-spacing:.2px}}
.rym-group-title{{font:320 16px 'consolaslxgw','Consolas NF + LXGW WenKai Mono','LXGW WenKai Mono','Microsoft YaHei',sans-serif;fill:#8f61b8;text-anchor:middle;letter-spacing:.3px}}
.rym-node-text{{font:320 16px 'consolaslxgw','Consolas NF + LXGW WenKai Mono','LXGW WenKai Mono','Microsoft YaHei',sans-serif;fill:#fff;text-anchor:middle;letter-spacing:.2px}}
</style></defs>"##
    );

    render_groups(&mut svg, graph);
    // Draw nodes first and edges second. Edge endpoints stop at node boundaries,
    // so this keeps short connectors and arrowheads visible instead of letting
    // an opaque node fill cover them.
    for node in &graph.nodes {
        render_node(&mut svg, node, &prefix);
    }
    let mut return_channel = 0_usize;
    for edge in &graph.edges {
        let edge_return_channel = return_channel;
        let return_plan = edge.is_return.then(|| {
            return_channel += 1;
            return_plan_for(graph, edge, edge_return_channel)
        });
        render_edge(
            &mut svg,
            graph,
            edge,
            return_plan,
            edge_return_channel,
            &prefix,
        );
    }
    svg.push_str("</svg>");
    svg
}

fn render_groups(svg: &mut String, graph: &Graph) {
    let group_map: HashMap<&str, &Group> = graph
        .groups
        .iter()
        .map(|group| (group.id.as_str(), group))
        .collect();
    let mut groups = graph.groups.iter().collect::<Vec<_>>();
    groups.sort_by_key(|group| (group.parent.is_some(), group.source_order));
    for group in groups {
        let members: Vec<&Node> = graph
            .nodes
            .iter()
            .filter(|node| {
                node.group.as_deref() == Some(group.id.as_str())
                    || node.group.as_deref().is_some_and(|child| {
                        group_map
                            .get(child)
                            .and_then(|candidate| candidate.parent.as_deref())
                            == Some(group.id.as_str())
                    })
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        let min_x = members.iter().map(|node| node.x).fold(f64::MAX, f64::min) - 24.0;
        let min_y = members.iter().map(|node| node.y).fold(f64::MAX, f64::min) - 34.0;
        let max_x = members
            .iter()
            .map(|node| node.x + node.width)
            .fold(0.0_f64, f64::max)
            + 24.0;
        let max_y = members
            .iter()
            .map(|node| node.y + node.height)
            .fold(0.0_f64, f64::max)
            + 24.0;
        let _ = write!(
            svg,
            r##"<g class="rym-group"><rect x="{min_x:.1}" y="{min_y:.1}" width="{:.1}" height="{:.1}" rx="{GROUP_CORNER_RADIUS:.1}" fill="#a566ff" fill-opacity=".08" stroke="#8f61b8" stroke-opacity=".78" stroke-width="1.8" stroke-dasharray="9 7"/><text class="rym-group-title" x="{:.1}" y="{:.1}">{}</text></g>"##,
            max_x - min_x,
            max_y - min_y,
            (min_x + max_x) / 2.0,
            min_y + 29.0,
            escape_xml(&group.label)
        );
    }
}

fn render_edge(
    svg: &mut String,
    graph: &Graph,
    edge: &Edge,
    return_plan: Option<ReturnPlan>,
    return_channel: usize,
    prefix: &str,
) {
    let from = &graph.nodes[edge.from];
    let to = &graph.nodes[edge.to];
    let (path, label_x, label_y) = if edge.from == edge.to {
        let start_x = from.x + from.width;
        let start_y = from.y + from.height * 0.38;
        let loop_x = start_x + 42.0;
        let end_y = from.y + from.height * 0.68;
        (
            format!(
                "M {start_x:.1} {start_y:.1} C {loop_x:.1} {start_y:.1}, {loop_x:.1} {end_y:.1}, {start_x:.1} {end_y:.1}"
            ),
            loop_x + 4.0,
            (start_y + end_y) / 2.0,
        )
    } else if edge.is_return {
        let plan = return_plan.unwrap_or(ReturnPlan {
            start: ReturnPort::Left,
            end: ReturnPort::Right,
        });
        let return_path = return_path_for(graph, edge, plan, return_channel);
        (
            format!(
                "M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
                return_path.start_x,
                return_path.start_y,
                return_path.control1_x,
                return_path.control1_y,
                return_path.control2_x,
                return_path.control2_y,
                return_path.end_x,
                return_path.end_y,
            ),
            (return_path.start_x + return_path.end_x) / 2.0,
            (return_path.control1_y + return_path.control2_y) / 2.0,
        )
    } else if let Some(route) = forward_bypass_route(graph, edge) {
        (
            rounded_polyline_path(&route.points, 18.0),
            route.label_x,
            route.label_y,
        )
    } else {
        match graph.direction {
            Direction::LeftToRight => {
                let from_center_x = from.x + from.width / 2.0;
                let to_center_x = to.x + to.width / 2.0;
                let same_column = (from_center_x - to_center_x).abs()
                    <= (from.width.min(to.width) * 0.35).max(18.0);
                if same_column {
                    let going_up = to.y + to.height / 2.0 < from.y + from.height / 2.0;
                    let start_x = from_center_x;
                    let start_y = if going_up {
                        from.y
                    } else {
                        from.y + from.height
                    };
                    let end_x = to_center_x;
                    let end_y = if going_up { to.y + to.height } else { to.y };
                    let bend_y = if going_up {
                        start_y - 28.0
                    } else {
                        start_y + 28.0
                    };
                    (
                        format!(
                            "M {start_x:.1} {start_y:.1} C {start_x:.1} {bend_y:.1}, {end_x:.1} {bend_y:.1}, {end_x:.1} {end_y:.1}"
                        ),
                        (start_x + end_x) / 2.0 + 18.0,
                        (start_y + end_y) / 2.0,
                    )
                } else {
                    // Wrapped snake rows run from right to left. Use the
                    // source's left port and target's right port in that
                    // direction so the connector stays in the gap instead
                    // of crossing through either node.
                    let going_left = to_center_x < from_center_x;
                    let start_x = if going_left {
                        from.x
                    } else {
                        from.x + from.width
                    };
                    let start_y = from.y + from.height / 2.0;
                    let end_x = if going_left { to.x + to.width } else { to.x };
                    let end_y = to.y + to.height / 2.0;
                    let middle_x = (start_x + end_x) / 2.0;
                    (
                        format!(
                            "M {start_x:.1} {start_y:.1} C {middle_x:.1} {start_y:.1}, {middle_x:.1} {end_y:.1}, {end_x:.1} {end_y:.1}"
                        ),
                        middle_x,
                        (start_y + end_y) / 2.0 - 12.0,
                    )
                }
            }
            Direction::TopDown => {
                let start_x = from.x + from.width / 2.0;
                let start_y = from.y + from.height;
                let end_x = to.x + to.width / 2.0;
                let end_y = to.y;
                let middle_y = (start_y + end_y) / 2.0;
                (
                    format!(
                        "M {start_x:.1} {start_y:.1} C {start_x:.1} {middle_y:.1}, {end_x:.1} {middle_y:.1}, {end_x:.1} {end_y:.1}"
                    ),
                    (start_x + end_x) / 2.0 + 10.0,
                    middle_y,
                )
            }
        }
    };

    let mut classes = String::from("rym-edge");
    if edge.dashed || edge.is_return {
        classes.push_str(" rym-return");
    }
    let marker = if edge.arrow {
        format!(" marker-end=\"url(#{prefix}-arrow)\"")
    } else {
        String::new()
    };
    let _ = write!(
        svg,
        r#"<path class="{classes}" d="{path}"{marker} data-edge="{}"/>"#,
        edge.source_order
    );
    if !edge.label.is_empty() {
        let _ = write!(
            svg,
            r##"<text class="rym-edge-label" x="{label_x:.1}" y="{label_y:.1}">{}</text>"##,
            escape_xml(&edge.label)
        );
    }
}

fn render_node(svg: &mut String, node: &Node, prefix: &str) {
    let (fill, stroke, glow) = match node.tone {
        NodeTone::Purple => ("#21172b", "#a566ff", "purple-glow"),
        NodeTone::Orange => ("#352812", "#ff741c", "orange-glow"),
        NodeTone::Green => ("#10291e", "#35d483", "green-glow"),
    };
    let _ = write!(
        svg,
        r#"<g class="rym-node" data-node-id="{}" filter="url(#{prefix}-{glow})">"#,
        escape_xml(&node.id)
    );
    match node.kind {
        NodeKind::Decision => {
            let x = node.x;
            let y = node.y;
            let width = node.width;
            let height = node.height;
            let center_x = x + width / 2.0;
            let center_y = y + height / 2.0;
            let path = format!(
                "M {center_x:.1} {y:.1} L {:.1} {center_y:.1} L {center_x:.1} {:.1} L {x:.1} {center_y:.1} Z",
                x + width,
                y + height,
            );
            let _ = write!(
                svg,
                r##"<path d="{path}" fill="{fill}" fill-opacity=".94" stroke="{stroke}" stroke-width="2.3" stroke-linejoin="round"/>"##
            );
        }
        NodeKind::Terminal => {
            let _ = write!(
                svg,
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" fill="{fill}" fill-opacity=".94" stroke="{stroke}" stroke-width="2.3"/>"##,
                node.x,
                node.y,
                node.width,
                node.height,
                node.height / 2.0
            );
        }
        NodeKind::Process => {
            let _ = write!(
                svg,
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{PROCESS_CORNER_RADIUS:.1}" fill="{fill}" fill-opacity=".94" stroke="{stroke}" stroke-width="2.3"/>"##,
                node.x, node.y, node.width, node.height,
            );
        }
    }

    let lines = wrap_label(&node.label, 16);
    let line_height = 20.0;
    let first_y = node.y + node.height / 2.0
        - (lines.len().saturating_sub(1) as f64 * line_height) / 2.0
        + 5.0;
    let _ = write!(
        svg,
        r#"<text class="rym-node-text" x="{:.1}" y="{first_y:.1}">"#,
        node.x + node.width / 2.0
    );
    for (line_index, line) in lines.iter().enumerate() {
        let dy = if line_index == 0 { 0.0 } else { line_height };
        let _ = write!(
            svg,
            r#"<tspan x="{:.1}" dy="{dy:.1}">{}</tspan>"#,
            node.x + node.width / 2.0,
            escape_xml(line)
        );
    }
    svg.push_str("</text></g>");
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash = 2_166_136_261_u32;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn truncate(value: &str, limit: usize) -> String {
    let mut result: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_renders_cycle() {
        let source = r#"flowchart LR
            A([开始]) --> B[收集需求]
            B --> C{审核通过?}
            C -->|是| D[并行开发]
            C -->|否| B
            D --> E([发布])"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let svg = render_svg(&graph, source);
        assert!(svg.contains("data-ryen-renderer=\"0.1.0\""));
        assert!(svg.contains("rym-return"));
        assert!(svg.contains("审核通过?"));
    }

    #[test]
    fn parses_standard_chained_edges_without_phantom_nodes() {
        let source = r#"flowchart LR
            A([开始]) --> B[准备输入] --> C{资料完整?}
            C -->|是| D[执行处理] --> E([完成])
            C -.-> F[可选复核] -- 通过 --> E"#;
        let graph = parse_graph(source).expect("parse");
        let ids = graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(ids, HashSet::from(["A", "B", "C", "D", "E", "F"]));
        assert_eq!(graph.edges.len(), 6);
        assert!(
            graph
                .nodes
                .iter()
                .all(|node| !node.label.contains("-->") && !node.label.starts_with('>'))
        );
        assert!(graph.edges.iter().any(|edge| edge.dashed));
        assert!(graph.edges.iter().any(|edge| edge.label == "通过"));
    }

    #[test]
    fn top_down_skip_edge_uses_clear_outer_lane() {
        let source = r#"flowchart TD
            A([提交申请]) --> B[自动校验资料]
            B --> C{资料是否完整?}
            C -->|是| E[进入人工审核]
            E --> F{风险等级}
            F -->|低| G[自动批准]
            F -->|中| H[主管复核]
            F -->|高| I[风控委员会评审]
            H --> J{复核通过?}
            I --> J
            J -->|是| G
            G --> K([完成])"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let edge = graph
            .edges
            .iter()
            .find(|edge| graph.nodes[edge.from].id == "F" && graph.nodes[edge.to].id == "G")
            .expect("low-risk skip edge");
        let review = graph
            .nodes
            .iter()
            .find(|node| node.id == "J")
            .expect("review decision");
        let direct = normal_edge_curve(&graph, edge);
        assert!(return_path_hits_node(&direct, review));

        let route = forward_bypass_route(&graph, edge).expect("outer bypass route");
        assert_eq!(route.side, ForwardBypassSide::Left);
        for (index, node) in graph.nodes.iter().enumerate() {
            if index != edge.from && index != edge.to {
                assert!(
                    !polyline_hits_rect(
                        &route.points,
                        node.x - 10.0,
                        node.y - 10.0,
                        node.x + node.width + 10.0,
                        node.y + node.height + 10.0,
                    ),
                    "bypass intersects {}",
                    node.id
                );
            }
        }
        assert!(rounded_polyline_path(&route.points, 18.0).contains(" Q "));
    }

    #[test]
    fn rejects_sequence_diagram_for_native_fallback() {
        assert!(parse_graph("sequenceDiagram\nA->>B: hello").is_err());
    }

    #[test]
    fn applies_ryen_classes_and_balances_decision_nodes() {
        let source = r#"flowchart LR
            S([开始]) --> D{需求完整?}
            D -->|否| R[补充信息]
            D -->|是| P[方案设计]
            class S ryen-success;
            class D ryen-decision;
            class R ryen-rework;
            class P ryen-process;"#;
        let graph = parse_graph(source).expect("parse");
        let decision = graph
            .nodes
            .iter()
            .find(|node| node.id == "D")
            .expect("decision");
        let rework = graph
            .nodes
            .iter()
            .find(|node| node.id == "R")
            .expect("rework");
        let success = graph
            .nodes
            .iter()
            .find(|node| node.id == "S")
            .expect("success");
        assert_eq!(decision.kind, NodeKind::Decision);
        assert_eq!(decision.tone, NodeTone::Orange);
        assert!(decision.height >= decision.width * 0.68);
        assert_eq!(rework.kind, NodeKind::Process);
        assert_eq!(rework.tone, NodeTone::Purple);
        assert!(rework.is_rework);
        assert_eq!(success.kind, NodeKind::Terminal);
        assert_eq!(success.tone, NodeTone::Green);
    }

    #[test]
    fn semantic_node_types_control_shape_and_color() {
        let source = r#"flowchart LR
            S[开始] --> P[构建完成]
            subgraph G[处理分组]
                P --> D{检查通过?}
                D -->|否| R[回滚并分析]
            end
            D -->|是| E[结束]
            class P ryen-success;
            class R ryen-rework;"#;
        let mut graph = parse_graph(source).expect("parse");
        let node = |id: &str| graph.nodes.iter().find(|node| node.id == id).expect("node");
        assert_eq!(node("S").kind, NodeKind::Terminal);
        assert_eq!(node("S").tone, NodeTone::Green);
        assert_eq!(node("E").kind, NodeKind::Terminal);
        assert_eq!(node("E").tone, NodeTone::Green);
        assert_eq!(node("P").kind, NodeKind::Process);
        assert_eq!(node("P").tone, NodeTone::Purple);
        assert_eq!(node("R").kind, NodeKind::Process);
        assert_eq!(node("R").tone, NodeTone::Purple);
        assert!(node("R").is_rework);
        assert_eq!(node("D").kind, NodeKind::Decision);
        assert_eq!(node("D").tone, NodeTone::Orange);

        layout_graph(&mut graph).expect("layout");
        let svg = render_svg(&graph, source);
        assert!(svg.contains("rx=\"12.0\" fill=\"#21172b\""));
        assert!(svg.contains("stroke-dasharray=\"9 7\""));
        assert!(svg.contains("fill=\"#a566ff\" fill-opacity=\".08\""));
    }

    #[test]
    fn branch_lanes_do_not_push_wrapped_main_flow_apart() {
        let source = r#"flowchart LR
            S([开始]) --> R[收集需求]
            R --> RD{需求完整?}
            RD -->|否| RF[补充信息]
            RD -->|是| PLAN[方案设计]
            PLAN --> REVIEW[综合评审]
            REVIEW --> AD{评审通过?}
            AD -->|否| AF[修改方案]
            AD -->|是| DEV[开发实施]
            DEV --> TD{测试通过?}
            TD -->|否| TF[修复问题]
            TD -->|是| PUB[发布上线]
            PUB --> ARCHIVE[复盘归档]
            ARCHIVE --> END([结束])
            class RF,AF,TF ryen-rework;"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let node = |id: &str| graph.nodes.iter().find(|node| node.id == id).expect("node");
        let request = node("R");
        let first_rework = node("RF");
        let decision = node("TD");
        let last_rework = node("TF");
        let development = node("DEV");
        assert!(first_rework.y + first_rework.height <= request.y);
        assert!(last_rework.y >= decision.y + decision.height);
        assert!(development.y - request.y < 320.0);
    }

    #[test]
    fn short_graph_keeps_rework_nodes_on_canvas() {
        let source = r#"flowchart LR
            S([开始]) --> D{需求完整?}
            D -->|否| R[补充信息]
            D -->|是| P[方案设计]
            class R ryen-rework;"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let rework = graph
            .nodes
            .iter()
            .find(|node| node.id == "R")
            .expect("rework");
        assert!(rework.x > 0.0 && rework.y > 0.0);
    }

    #[test]
    fn rework_annotations_become_real_dashed_return_edges() {
        let source = r#"flowchart LR
            S([开始]) --> R[收集需求]
            R --> D{需求完整?}
            D -->|否| RF[补充信息<br/>返回：收集需求]
            D -->|是| P[方案设计]
            class RF ryen-rework;"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let rework = graph
            .nodes
            .iter()
            .find(|node| node.id == "RF")
            .expect("rework");
        assert_eq!(rework.label, "补充信息");
        let return_edge = graph
            .edges
            .iter()
            .find(|edge| edge.from == rework.source_order && edge.to != rework.source_order)
            .expect("synthetic return edge");
        assert!(return_edge.is_return);
        assert!(return_edge.dashed);
        let svg = render_svg(&graph, source);
        assert!(svg.contains("rym-return"));
    }

    #[test]
    fn adaptive_rework_routes_choose_clear_side_ports() {
        let source = r#"flowchart LR
            S([开始]) --> R[收集需求]
            R --> RD{需求完整?}
            RD -->|否| RF[补充信息<br/>返回：收集需求]
            RD -->|是| PLAN[方案设计]
            PLAN --> REVIEW[综合评审]
            REVIEW --> AD{评审通过?}
            AD -->|否| AF[修改方案<br/>返回：方案设计]
            AD -->|是| DEV[开发实施]
            DEV --> TD{测试通过?}
            TD -->|否| TF[修复问题<br/>返回：开发实施]
            TD -->|是| PUB[发布上线]
            PUB --> ARCHIVE[复盘归档]
            ARCHIVE --> END([结束])
            class RF,AF,TF ryen-rework;"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let plan_for = |from: &str, to: &str| {
            let edge = graph
                .edges
                .iter()
                .find(|edge| graph.nodes[edge.from].id == from && graph.nodes[edge.to].id == to)
                .expect("return edge");
            return_plan_for(&graph, edge, 0)
        };
        let first = plan_for("RF", "R");
        let second = plan_for("AF", "PLAN");
        let third = plan_for("TF", "DEV");
        assert_eq!(first.start, ReturnPort::Left);
        assert_eq!(first.end, ReturnPort::Top);
        assert_eq!(second.start, ReturnPort::Left);
        assert_eq!(third.start, ReturnPort::Right);
        assert_eq!(third.end, ReturnPort::Right);
    }

    #[test]
    fn top_down_subgraphs_share_outer_rank_without_turning_internal_edges_into_returns() {
        let source = r#"flowchart LR
            PLAN[方案设计] --> UI
            PLAN --> TECH
            subgraph PE[并行评估]
                direction TB
                UI[UI 原型] --> TECH[技术评审]
            end
            UI --> REVIEW[综合评审]
            TECH --> REVIEW"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let ui = graph.nodes.iter().find(|node| node.id == "UI").expect("UI");
        let tech = graph
            .nodes
            .iter()
            .find(|node| node.id == "TECH")
            .expect("TECH");
        assert_eq!(ui.rank, tech.rank);
        assert!(tech.y > ui.y, "UI y={} TECH y={}", ui.y, tech.y);
        let internal = graph
            .edges
            .iter()
            .find(|edge| graph.nodes[edge.from].id == "UI" && graph.nodes[edge.to].id == "TECH")
            .expect("internal edge");
        assert!(!internal.is_return);
    }

    #[test]
    fn sibling_top_down_subgraphs_are_separate_visual_blocks() {
        let source = r#"flowchart LR
            START([需求进入]) --> SPLIT{是否需要双线评估?}
            subgraph PRODUCT[产品工作区]
                direction TB
                P1[需求拆解] --> P2[交互原型]
                P2 --> P3[产品验收]
            end
            subgraph ENGINEERING[工程工作区]
                direction TB
                E1[技术预研] --> E2[接口设计]
                E2 --> E3[工程评审]
            end
            SPLIT --> P1
            SPLIT --> E1
            P3 --> MERGE[合并评审结果]
            E3 --> MERGE"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");

        let group_bounds = |group_id: &str| {
            let members = graph
                .nodes
                .iter()
                .filter(|node| node.group.as_deref() == Some(group_id))
                .collect::<Vec<_>>();
            let min_y = members.iter().map(|node| node.y).fold(f64::MAX, f64::min) - 34.0;
            let max_y = members
                .iter()
                .map(|node| node.y + node.height)
                .fold(0.0_f64, f64::max)
                + 24.0;
            (min_y, max_y)
        };
        let product = group_bounds("PRODUCT");
        let engineering = group_bounds("ENGINEERING");
        assert!(
            product.1 <= engineering.0 || engineering.1 <= product.0,
            "group rectangles overlap: product={product:?} engineering={engineering:?}"
        );
    }

    #[test]
    fn top_down_return_curves_avoid_unrelated_nodes() {
        let source = r#"flowchart TB
            S([构建完成]) --> T[执行自动化测试]
            T --> Q{全部通过?}
            Q -->|否| FIX[修复失败用例<br/>返回：执行自动化测试]
            Q -->|是| PKG[生成候选版本]
            PKG --> CANARY[小流量灰度]
            CANARY --> METRIC{核心指标正常?}
            METRIC -->|否| ROLLBACK[回滚并分析<br/>返回：生成候选版本]
            METRIC -->|是| EXPAND[扩大灰度范围]
            EXPAND --> FEEDBACK{收到严重反馈?}
            FEEDBACK -->|是| HOTFIX[准备热修复<br/>返回：执行自动化测试]
            FEEDBACK -->|否| RELEASE[全量发布]"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");

        let mut channel = 0_usize;
        for edge in graph.edges.iter().filter(|edge| edge.is_return) {
            let plan = return_plan_for(&graph, edge, channel);
            let path = return_path_for(&graph, edge, plan, channel);
            for (index, node) in graph.nodes.iter().enumerate() {
                if index != edge.from && index != edge.to {
                    assert!(
                        !return_path_hits_node(&path, node),
                        "return {} -> {} intersects {} with {plan:?}",
                        graph.nodes[edge.from].id,
                        graph.nodes[edge.to].id,
                        node.id,
                    );
                }
            }
            channel += 1;
        }
    }

    #[test]
    fn return_curves_add_body_only_when_perpendicular_span_is_long() {
        let perpendicular = ReturnPlan {
            start: ReturnPort::Left,
            end: ReturnPort::Top,
        };
        let compact = return_curve_handle_length(0.0, 0.0, 80.0, 80.0, perpendicular, 0);
        let medium = return_curve_handle_length(0.0, 0.0, 150.0, 80.0, perpendicular, 0);
        let long = return_curve_handle_length(0.0, 0.0, 450.0, 80.0, perpendicular, 0);
        assert_eq!(compact, 32.0);
        assert!(medium > compact);
        assert!(long > medium);
        assert!(long <= 160.0);
    }

    #[test]
    fn horizontal_edges_keep_visible_strokes() {
        let source = r#"flowchart LR
            A[开始] --> B[下一步]"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let svg = render_svg(&graph, source);
        assert!(svg.contains("stroke-width:3.4"));
        assert!(!svg.contains(".rym-edge{fill:none;stroke:#a566ff;stroke-width:3.4;stroke-linecap:round;stroke-linejoin:round;filter:"));
        assert_eq!(svg.matches("data-edge=").count(), 1);
    }

    #[test]
    fn wraps_long_left_to_right_graphs_without_special_case_node_ids() {
        let source = r#"flowchart LR
            A([开始]) --> B[收集需求]
            B --> C{需求完整?}
            C --> D[方案设计]
            D --> E[综合评审]
            E --> F{评审通过?}
            F --> G[开发实施]
            G --> H{测试通过?}
            H --> I[发布上线]
            I --> J[复盘归档]
            J --> K([结束])"#;
        let mut graph = parse_graph(source).expect("parse");
        layout_graph(&mut graph).expect("layout");
        let node_right = graph
            .nodes
            .iter()
            .map(|node| node.x + node.width)
            .fold(0.0_f64, f64::max);
        let node_bottom = graph
            .nodes
            .iter()
            .map(|node| node.y + node.height)
            .fold(0.0_f64, f64::max);
        let svg = render_svg(&graph, source);
        assert!(node_right + MARGIN <= MAX_LAYOUT_WIDTH);
        assert!(node_bottom > MARGIN + NODE_HEIGHT + ROW_GAP);
        assert_eq!(svg.matches("data-edge=").count(), 10);

        let reverse = graph
            .edges
            .iter()
            .find(|edge| {
                if edge.is_return {
                    return false;
                }
                let from = &graph.nodes[edge.from];
                let to = &graph.nodes[edge.to];
                let from_center = from.x + from.width / 2.0;
                let to_center = to.x + to.width / 2.0;
                to_center < from_center
                    && (from_center - to_center).abs() > (from.width.min(to.width) * 0.35).max(18.0)
            })
            .expect("wrapped graph has a right-to-left edge");
        let from = &graph.nodes[reverse.from];
        let expected_start = format!("M {:.1} {:.1} C", from.x, from.y + from.height / 2.0);
        assert!(svg.contains(&expected_start));
    }
}
