use clap::{App, Arg, ArgMatches};

pub struct Configs {
    pub prog_name: String,
    pub list: bool,
    pub device_index: u32,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub validation: bool,
    pub debug_mode: bool,
    pub shader_dir: String,
    pub mesh_file: String,
}

impl Configs {
    pub fn new(name: &'static str) -> Configs {
        Configs::from(name, create_app(name).get_matches())
    }

    fn from<'a, 'b>(name: &'static str, args: ArgMatches<'a>) -> Configs {
        Configs {
            prog_name: String::from(name),
            list: args.is_present("list"),
            device_index: args.value_of("device").unwrap_or("0").parse().unwrap_or(0),
            width: args.value_of("width").unwrap_or("0").parse().unwrap_or(0),
            height: args.value_of("height").unwrap_or("0").parse().unwrap_or(0),
            fullscreen: args.is_present("fullscreen"),
            validation: args.is_present("validation"),
            debug_mode: args.is_present("debug"),
            shader_dir: args.value_of("shader").unwrap_or("").to_string(),
            mesh_file: args.value_of("mesh").unwrap_or("").to_string()
        }
    }
}

fn create_app<'a, 'b, S: Into<String>>(name: S) -> App<'a, 'b> {
    App::new(name)
        .arg(
            Arg::with_name("list")
                .long("list")
                .short("l")
                .help("show all available devices"),
        )
        .arg(
            Arg::with_name("device")
                .long("device")
                .short("d")
                .help("use specific device")
                .default_value("0"),
        )
        .arg(
            Arg::with_name("validation")
                .long("validation")
                .short("v")
                .help("use VK_LAYER_LUNARG_standard_validation"),
        )
        .arg(
            Arg::with_name("width")
                .long("width")
                .short("w")
                .help("window width")
                .default_value("640"),
        )
        .arg(
            Arg::with_name("height")
                .long("height")
                .short("h")
                .help("window height")
                .default_value("480"),
        )
        .arg(
            Arg::with_name("fullscreen")
                .long("fullscreen")
                .short("f")
                .help("fullscreen"),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .short("g")
                .help("debug mode"),
        )
        .arg(
            Arg::with_name("shader")
                .long("shader")
                .short("s")
                .help("shader directory")
                .default_value("../shaders/"),
        )
        .arg(
            Arg::with_name("mesh")
                .long("mesh")
                .short("m")
                .help("mesh file")
                .default_value("../mesh/sponza.dae"),
        )
}
