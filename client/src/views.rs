use crate::file::Cluster;
use cursive::view::ViewWrapper;
use cursive::views::{Dialog, SelectView, TextView};
use cursive::wrap_impl;

pub(crate) struct ClusterSelectView {
    view: SelectView<usize>,
    pub(crate) clusters: Vec<Cluster>,
}

impl ClusterSelectView {
    pub(crate) fn new(clusters: Vec<Cluster>) -> Self {
        let index_width = ((clusters.len() + 1) as f64).log10() as usize + 1;
        let mut view = SelectView::<usize>::new().with_all(
            clusters
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

        ClusterSelectView { view, clusters }
    }
}

impl ViewWrapper for ClusterSelectView {
    wrap_impl!(self.view: SelectView<usize>);
}
