use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::{SizeConstraint, ViewWrapper};
use cursive::views::{BoxView, Dialog, DummyView, LinearLayout, Panel, SelectView, TextView};
use cursive::wrap_impl;

use std::io;
use std::path::{Path, PathBuf};

use crate::models::{Cluster, ClusterSet};

pub(crate) struct MainView {
    view: LinearLayout,
}

impl MainView {
    pub(crate) fn from_paths<P: AsRef<Path>>(model: P, clusters: P, raw: P) -> io::Result<Self> {
        let cluster_select = ClusterSelectView::from_paths(model, clusters, raw)?;
        let quit_view = TextView::new("Press q to exit.".to_string());
        let save_view = TextView::new(
            "Press w to write the signatures of clusters qualified as benign into a file."
                .to_string(),
        );
        let top_layout = LinearLayout::new(Orientation::Vertical)
            .child(
                cluster_select
                    .with_id("cluster_select")
                    .scrollable()
                    .full_width()
                    .fixed_height(30),
            )
            .child(DummyView)
            .child(DummyView)
            .child(quit_view)
            .child(save_view);

        let cluster_prop_box1 =
            BoxView::new(SizeConstraint::Full, SizeConstraint::Full, top_layout)
                .with_id("cluster_view");
        let cluster_prop_box2 = BoxView::new(
            SizeConstraint::Fixed(60),
            SizeConstraint::Fixed(60),
            Panel::new(
                LinearLayout::vertical()
                    .child(TextView::new("").with_id("cluster_properties"))
                    .child(
                        Dialog::around(TextView::new(
                            "Please Select a cluster from the left lists",
                        ))
                        .with_id("cluster_properties2"),
                    ),
            ),
        );
        let mut view = LinearLayout::horizontal();
        view.add_child(cluster_prop_box1);
        view.add_child(cluster_prop_box2);
        Ok(MainView { view })
    }
}

impl ViewWrapper for MainView {
    wrap_impl!(self.view: LinearLayout);
}

pub(crate) struct ClusterSelectView {
    view: SelectView<usize>,
    pub(crate) clusters_path: PathBuf,
    pub(crate) clusters: ClusterSet,
}

impl ClusterSelectView {
    pub(crate) fn from_paths<P: AsRef<Path>>(model: P, clusters: P, raw: P) -> io::Result<Self> {
        let clusters_path = clusters.as_ref().to_path_buf();
        let clusters = ClusterSet::from_paths(model, clusters, raw)?;

        let index_width = ((clusters.len() + 1) as f64).log10() as usize + 1;
        let mut view = SelectView::<usize>::new().with_all(
            clusters
                .clusters
                .iter()
                .map(|c| &c.signature)
                .enumerate()
                .map(|(i, label)| {
                    let index_str = (i + 1).to_string();
                    (
                        " ".repeat(index_width - index_str.len()) + &index_str + " " + label,
                        i,
                    )
                }),
        );

        view.set_on_submit(|siv, &i| {
            let mut cluster_prop_window1 = siv.find_id::<TextView>("cluster_properties").unwrap();
            let properties = siv
                .call_on_id("cluster_select", |view: &mut ClusterSelectView| {
                    Cluster::get_cluster_properties(&view.clusters[i])
                })
                .unwrap();
            cluster_prop_window1.set_content(properties);

            let mut cluster_prop_window2 = siv.find_id::<Dialog>("cluster_properties2").unwrap();
            let mut qualifier_select = SelectView::new();
            qualifier_select.add_item("Suspicious".to_string(), 1);
            qualifier_select.add_item("Benign".to_string(), 2);
            qualifier_select.add_item("None".to_string(), 3);
            cluster_prop_window2.set_content(qualifier_select.on_submit(
                move |siv, qualifier_val| {
                    let qualifier = match qualifier_val {
                        1 => "Suspicious".to_string(),
                        2 => "Benign".to_string(),
                        _ => "None".to_string(),
                    };
                    siv.call_on_id("cluster_select", |view: &mut ClusterSelectView| {
                        if view.clusters[i].suspicious != qualifier {
                            view.clusters[i].suspicious = qualifier;
                        }
                    });
                },
            ));
        });

        Ok(ClusterSelectView {
            view,
            clusters_path,
            clusters,
        })
    }
}

impl ViewWrapper for ClusterSelectView {
    wrap_impl!(self.view: SelectView<usize>);
}

pub(crate) fn bin2str(bin: &[u8]) -> String {
    bin.iter()
        .map(|&c| {
            if 0x20 <= c && c <= 0x7e {
                c as char
            } else {
                '.'
            }
        })
        .collect::<String>()
}
