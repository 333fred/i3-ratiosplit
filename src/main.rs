#[macro_use]
extern crate log;

use core::panic;
use std::fs::OpenOptions;

use i3ipc::{
    event::{inner::WindowChange, Event, WindowEventInfo},
    reply::{Node, NodeLayout, NodeType},
    EstablishError, I3Connection, I3EventListener, Subscription,
};
use log::{trace, warn};
use settings::{load_settings, Settings};
use simplelog::{CombinedLogger, SharedLogger, TermLogger, TerminalMode, WriteLogger};

mod settings;

fn main() {
    let settings = load_settings();
    setup_logger(&settings);

    info!("Starting i3 ratiosplit, connecting to i3");

    let (mut connection, mut listener) = match setup_i3_connection() {
        Ok(t) => t,
        Err(error) => {
            error!("Error connecting to i3: {:?}", error);
            return;
        }
    };

    let events = [Subscription::Window];
    info!("Subscribing to events: {:?}", events);
    if let Err(error) = listener.subscribe(&events) {
        error!("Error subscribing to events: {:?}", error);
        return;
    }

    for event in listener.listen() {
        if let Ok(Event::WindowEvent(event_info)) = event {
            match event_info {
                WindowEventInfo {
                    change: WindowChange::New,
                    container,
                } => {
                    info!("New window created {:?}", container.name);
                    trace!("Container properties: {:?}", container);
                    handle_child(&mut connection, container);
                }
                _ => {
                    trace!(
                        "Ignoring event {:?}: {:?}",
                        event_info.change, event_info.container.name
                    );
                }
            }
        } else {
            error!("Unexpected event or error: {:?}", event);
            return;
        }
    }
}

fn setup_logger(settings: &Settings) {
    let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::new();

    if let Ok(file) = OpenOptions::new()
        .append(true)
        .create(true)
        .open(settings.log_file.as_str())
    {
        loggers.push(WriteLogger::new(
            settings.log_file_level,
            simplelog::Config::default(),
            file,
        ))
    }

    if let Some(console) = TermLogger::new(
        settings.log_console_level,
        simplelog::Config::default(),
        TerminalMode::Mixed,
    ) {
        loggers.push(console);
    }

    CombinedLogger::init(loggers).unwrap();

    info!("Using settings {:?}", settings);
}

fn setup_i3_connection() -> Result<(I3Connection, I3EventListener), EstablishError> {
    info!("Main connection connecting");
    let connection = I3Connection::connect()?;
    info!("Listener connecting");
    let listener = I3EventListener::connect()?;
    Ok((connection, listener))
}

fn handle_child(connection: &mut I3Connection, new_node: Node) {
    trace!("Retreiving current tree");

    let tree = match connection.get_tree() {
        Ok(t) => t,
        Err(error) => {
            error!("Error retreiving the current i3 tree: {:?}", error);
            panic!("Error retreiving the current i3 tree: {:?}", error);
        }
    };

    trace!("Retrieved tree.");

    if let Some(parent) = find_parent(new_node.id, &tree) {
        trace!("Found parent node for {:?}", new_node.name);

        // If the parent is not a container or is not a splitv/h, there's nothing to resize
        if !matches!(parent, Node { nodetype: NodeType::Con, layout: NodeLayout::SplitH, .. } |
                             Node { nodetype: NodeType::Con, layout: NodeLayout::SplitV, .. } |
                             Node { nodetype: NodeType::Workspace, layout: NodeLayout::SplitH, .. } |
                             Node { nodetype: NodeType::Workspace, layout: NodeLayout::SplitV, .. })
        {
            info!("Parent node is type {:?}, not resizing", parent.nodetype);
            trace!("Parent properties: {:?}", parent);
            return;
        }

        // If there are not 2 children in this node, we can't resize one for golden mode,
        // and would likely just annoy people if we did. Skip.
        if parent.nodes.len() != 2 {
            info!("Parent node has {} children, skipping", parent.nodes.len());
            trace!("Parent properties: {:?}", parent);
            return;
        }

        trace!("Parent node is of known config, resizing");

        // Finally, we want to resize the window, and set tiling to split the next window
        // in the opposite direction that this was split to maintain the golden spiral.
        // We actually set tiling first, on both windows, so that making a new window in either
        // location will correctly maintain the golden spiral. We then want to move the current
        // split location to 33% along the direction of the split.

        let resize_horizontal = parent.layout == NodeLayout::SplitH;

        trace!(
            "Resizing {}",
            if resize_horizontal {
                "horizontally"
            } else {
                "vertically"
            }
        );

        let split_command = format!(
            "split {}",
            if resize_horizontal {
                "vertical"
            } else {
                "horizontal"
            }
        );

        for child in &parent.nodes {
            let focus_child = focus_id(child);

            trace!("Running {}", focus_child);
            if let Err(error) = connection.run_command(focus_child.as_str()) {
                warn!("Error {:?} when focusing child {:?}", error, child);
                return;
            }

            trace!("Running {}", split_command);
            if let Err(error) = connection.run_command(split_command.as_str()) {
                warn!("Error {:?} when splitting child {:?}", error, child);
                return;
            }
        }

        trace!("Split children");

        let focus_command = format!("[id={}] focus", new_node.id);
        let resize_command = format!(
            "resize set {} 33 ppt",
            if resize_horizontal { "width" } else { "height" }
        );

        trace!("Running {}", focus_command);
        if let Err(error) = connection.run_command(focus_command.as_str()) {
            warn!("Error {:?} when focusing node {:?}", error, new_node);
            return;
        }

        trace!("Running {}", resize_command);
        if let Err(error) = connection.run_command(resize_command.as_str()) {
            warn!("Error {:?} when resizing node {:?}", error, new_node);
            return;
        }

        info!("Resized {:?} successfully", new_node.name);

        fn focus_id(node: &Node) -> String {
            format!("[id={}] focus", node.id)
        }
    } else {
        info!("Could not find parent node for {:?}.", new_node.name);
        trace!("Tree: {:?}", tree);
    }

    fn find_parent(child_id: i64, node: &Node) -> Option<&Node> {
        // In order to find the child node, we get the tree and loop through all the children.
        // There are a few possible failure conditions:
        // 1. The node isn't in the tree
        // 2. The node is a floating node (no need to dynamically resize these, so just don't check that field).
        // 3. The given id is for the root node.

        for child in &node.nodes {
            if child.id == child_id {
                return Some(node);
            } else if let Some(found) = find_parent(child_id, child) {
                return Some(found);
            }
        }

        None
    }
}
