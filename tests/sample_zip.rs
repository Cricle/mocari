use std::env;
use std::fs::File;
use std::io::Read;

use rusty_live2d::json::Model3;
use zip::ZipArchive;

#[test]
fn parses_model3_from_opt_in_sample_zip() {
    let Some(zip_path) = env::var_os("LIVE2D_SAMPLE_ZIP") else {
        eprintln!("skipping sample zip test; LIVE2D_SAMPLE_ZIP is not set");
        return;
    };

    let file = File::open(&zip_path).expect("open sample zip");
    let mut archive = ZipArchive::new(file).expect("read sample zip");
    let mut model_json = None;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("read zip entry");
        if !entry.name().ends_with(".model3.json") {
            continue;
        }

        let mut source = String::new();
        entry
            .read_to_string(&mut source)
            .expect("read model3 json from sample zip");
        model_json = Some(source);
        break;
    }

    let model_json = model_json.expect("sample zip contains a model3 json");
    let model = Model3::from_json_str(&model_json).expect("parse model3 json from sample zip");

    assert_eq!(model.version(), 3);
    assert!(!model.moc().is_empty());
    assert!(!model.textures().is_empty());
}
