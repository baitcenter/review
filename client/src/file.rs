use chrono::Utc;
use cursive::view::Position;
use cursive::views::{Dialog, TextView};
use cursive::Cursive;
use std::io;
use std::path::PathBuf;

use crate::models::write_clusters;
use crate::views::{ClusterSelectView, MainView};

pub struct ClusterView {
    cursive: Cursive,
}

impl ClusterView {
    pub fn new(cluster_path: &str, model_path: &str, raw_db_path: &str) -> io::Result<ClusterView> {
        let main_view = MainView::from_paths(model_path, cluster_path, raw_db_path)?;
        let mut cluster_view = ClusterView {
            cursive: Cursive::default(),
        };

        cluster_view.cursive.add_layer(main_view);
        cluster_view
            .cursive
            .add_global_callback('q', delete_and_quit);
        cluster_view
            .cursive
            .add_global_callback('w', write_benign_rules);

        Ok(cluster_view)
    }

    pub fn run_feedback_mode(&mut self) {
        self.cursive.run();
    }

    fn create_popup_window(cursive: &mut Cursive, popup_message: &str) {
        cursive.screen_mut().add_layer_at(
            Position::new(
                cursive::view::Offset::Center,
                cursive::view::Offset::Parent(5),
            ),
            Dialog::new()
                .content(TextView::new(popup_message))
                .dismiss_button("OK"),
        );
    }

    fn create_popup_window_then_quit(cursive: &mut Cursive, popup_message: &str) {
        cursive.screen_mut().add_layer_at(
            Position::new(
                cursive::view::Offset::Center,
                cursive::view::Offset::Parent(5),
            ),
            Dialog::new()
                .content(TextView::new(popup_message))
                .button("OK", Cursive::quit),
        );
    }
}

fn delete_and_quit(s: &mut Cursive) {
    s.screen_mut().add_layer_at(
        Position::new(cursive::view::Offset::Center, cursive::view::Offset::Parent(5)),
        Dialog::new()
            .content(TextView::new("Would you like to delete old events?\nThe most recent 25 events will be kept and the rest will be deleted from cluster file and database."))
            .dismiss_button("Back to the previous window")
            .button("No", Cursive::quit)
            .button("Yes", delete_old_events)
    );
}

fn delete_old_events(s: &mut Cursive) {
    match s
        .call_on_id(
            "cluster_select",
            |v: &mut ClusterSelectView| -> io::Result<()> {
                let clusters = v.clusters.delete_old_events().map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!("An error occurred while deleting events: {}", e),
                    )
                })?;
                write_clusters(&v.clusters_path, &clusters).map_err(|e| {
                    io::Error::new(e.kind(), format!("Writing clusters failed: {}", e))
                })?;
                Ok(())
            },
        )
        .unwrap()
    {
        Ok(_) => ClusterView::create_popup_window_then_quit(s, "Deleted successfully."),
        Err(e) => ClusterView::create_popup_window_then_quit(s, &format!("{}", e)),
    }
}

fn write_benign_rules(s: &mut Cursive) {
    let path = s
        .call_on_id("cluster_select", |v: &mut ClusterSelectView| {
            v.clusters_path.as_path().to_path_buf()
        })
        .unwrap();
    let mut path = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(PathBuf::default);
    path.push(Utc::now().format("%Y%m%d%H%M%S").to_string());
    path.push("_benign_rules.txt");

    match s
        .call_on_id("cluster_select", |v: &mut ClusterSelectView| {
            v.clusters.write_benign_rules(&path)
        })
        .unwrap()
    {
        Ok(_) => ClusterView::create_popup_window(
            s,
            &format!("Saved to {}.", path.as_os_str().to_string_lossy()),
        ),
        Err(e) => ClusterView::create_popup_window_then_quit(s, &format!("{}", e)),
    }
}
