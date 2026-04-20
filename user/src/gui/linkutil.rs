use egui::Ui;



pub fn https_link(ui: &mut Ui, rawlink: &str) {
    if rawlink.starts_with("https://") {
        ui.add(egui::Hyperlink::from_label_and_url(&rawlink[8..], &rawlink));
    } else {
        ui.add(egui::Hyperlink::from_label_and_url(rawlink, format!("https://{}", rawlink)));
    }
}