//  01_get_instance
use std::borrow::Cow;
use vk_sample_common::config;

fn main() {
    let config = config::Configs::new("get_instance");
    let app_info = vulkano::instance::ApplicationInfo {
        application_name: Some(Cow::from(config.prog_name.as_str())),
        application_version: Some(vulkano::instance::Version {
            major: 1,
            minor: 0,
            patch: 0,
        }),
        engine_name: Some(Cow::from("sample_engine")),
        engine_version: Some(vulkano::instance::Version {
            major: 1,
            minor: 0,
            patch: 0,
        }),
    };

    vulkano::instance::Instance::new(
        Some(&app_info),
        &vulkano::instance::InstanceExtensions::none(),
        if config.validation {
            vec!["VK_LAYER_LUNARG_standard_validation"]
        } else {
            Vec::<&str>::new()
        },
    )
    .unwrap();
}
