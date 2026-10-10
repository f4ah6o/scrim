//! CLI tool for inspecting accessibility trees and permissions.

use clap::{Parser, Subcommand};
use tracing::error;

#[cfg(target_os = "macos")]
use scrim_core::ElementFilter;
#[cfg(target_os = "macos")]
use tracing::info;

#[derive(Parser)]
#[command(name = "scrim")]
#[command(about = "Scrim - Keyboard-driven desktop navigation system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check accessibility permissions status
    Permissions,
    /// Dump the UI tree of the focused window
    Dump {
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
        /// Include all elements (not just interactive ones)
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Permissions => {
            check_permissions();
            Ok(())
        }
        Commands::Dump { format, all } => dump_ui_tree(&format, all).await,
    }
}

fn check_permissions() {
    #[cfg(target_os = "macos")]
    {
        use scrim_macos::permissions;

        let trusted = permissions::is_trusted();
        if trusted {
            println!("✓ Accessibility permissions: GRANTED");
            println!("  The application can access UI elements.");
        } else {
            println!("✗ Accessibility permissions: NOT GRANTED");
            println!("  To grant permissions:");
            println!("  1. Open System Settings > Privacy & Security > Accessibility");
            println!("  2. Add this application to the list");
            println!();
            println!("  Would you like to open the system prompt now? (y/N)");
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("ℹ Linux accessibility uses AT-SPI2");
        println!("  No special permissions are typically required on X11.");
        println!("  On Wayland, ensure AT-SPI2 is enabled in your desktop settings.");
    }

    #[cfg(target_os = "windows")]
    {
        println!("ℹ Windows accessibility uses UI Automation");
        println!("  No special permissions are typically required.");
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        println!("? Unknown platform");
    }
}

#[allow(unused_variables)]
async fn dump_ui_tree(format: &str, all: bool) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use scrim_core::AccessibilityClient;
        use scrim_macos::MacOSAccessibilityClient;

        let client = MacOSAccessibilityClient::new();

        // Check permissions first
        if !client.is_trusted() {
            error!("Accessibility permissions not granted. Run 'scrim permissions' for details.");
            anyhow::bail!("Permission denied");
        }

        // Get focused window
        info!("Getting focused window...");
        let root = client.get_focused_window().map_err(|e| {
            error!("Failed to get focused window: {}", e);
            anyhow::anyhow!("Failed to get focused window: {}", e)
        })?;

        // Traverse the tree
        let filter = if all {
            ElementFilter::All
        } else {
            ElementFilter::default()
        };

        info!("Traversing UI tree with filter: {:?}", filter);
        let elements = client.traverse_tree(&root, filter);

        match format {
            "json" => {
                let json = serde_json::to_string_pretty(&elements)?;
                println!("{}", json);
            }
            "text" | _ => {
                println!("UI Tree ({} elements found):", elements.len());
                println!();
                for (i, element) in elements.iter().enumerate() {
                    print_element(i, element, 0);
                }
            }
        }

        info!("Dumped {} elements", elements.len());
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        error!("UI tree dumping is only implemented on macOS");
        anyhow::bail!("Not implemented on this platform");
    }
}

#[cfg(target_os = "macos")]
fn print_element(index: usize, element: &scrim_core::UIElement, depth: usize) {
    let indent = "  ".repeat(depth);
    let label = element.label.as_deref().unwrap_or("");
    let actions = element.actions.join(", ");

    println!(
        "{}[{}] {} - \"{}\" ({}) at ({}, {}, {}x{})",
        indent,
        index,
        element.role,
        label,
        if actions.is_empty() {
            "no actions"
        } else {
            &actions
        },
        element.rect.x,
        element.rect.y,
        element.rect.width,
        element.rect.height,
    );

    for (i, child) in element.children.iter().enumerate() {
        print_element(i, child, depth + 1);
    }
}
