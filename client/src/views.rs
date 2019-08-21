use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::{SizeConstraint, ViewWrapper};
use cursive::views::{BoxView, Dialog, DummyView, LinearLayout, Panel, SelectView, TextView};
use cursive::wrap_impl;

use crate::error::Error;
use crate::models::{Cluster, ClusterSet};

pub(crate) struct MainView {
    view: LinearLayout,
}

impl MainView {
    pub(crate) fn from_reviewd(url: &str) -> Result<Self, Error> {
        let cluster_select = ClusterSelectView::from_reviewd(url)?;
        let quit_view = TextView::new("Press q to exit.".to_string());
        let save_view = TextView::new("Press s to save changes.".to_string());
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
            .child(save_view)
            .child(quit_view);

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
    pub(crate) clusters: ClusterSet,
}

impl ClusterSelectView {
    pub(crate) fn from_reviewd(url: &str) -> Result<Self, Error> {
        let clusters = ClusterSet::from_reviewd(url)?;

        let index_width = ((clusters.clusters.len() + 1) as f64).log10() as usize + 1;
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
            let qualifier_select = siv
                .call_on_id("cluster_select", |view: &mut ClusterSelectView| {
                    let mut qualifier_select = SelectView::new();
                    for (i, qualifier) in view.clusters.qualifier.iter() {
                        qualifier_select.add_item(qualifier.to_string(), *i);
                    }
                    qualifier_select
                })
                .unwrap();

            cluster_prop_window2.set_content(qualifier_select.on_submit(
                move |siv, qualifier_val| {
                    siv.call_on_id("cluster_select", |view: &mut ClusterSelectView| {
                        if view.clusters[i].qualifier != view.clusters.qualifier[qualifier_val] {
                            view.clusters[i].qualifier =
                                view.clusters.qualifier[qualifier_val].clone();
                            view.clusters
                                .updated_clusters
                                .insert(view.clusters[i].cluster_id.clone(), i);
                        }
                    });
                },
            ))
        });

        Ok(ClusterSelectView { view, clusters })
    }
}

impl ViewWrapper for ClusterSelectView {
    wrap_impl!(self.view: SelectView<usize>);
}
