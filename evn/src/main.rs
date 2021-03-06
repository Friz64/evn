use evn_engine::prelude::*;

fn main() {
    let mut game = Game::new(
        "Indev",
        |_world| {
            // register components here
        },
        |dispatcher| {
            // add systems here
            dispatcher
        },
        |res_builder| {
            res_builder
                .with_config(
                    "config",
                    "config.yml",
                    include_resource!(open: "config.yml"),
                )
                .with_shader(
                    "shader_normal",
                    "shaders/normal.vert.spv",
                    "shaders/normal.frag.spv",
                )
        },
        |window_builder| {
            window_builder
                .with_title("evn")
                .with_dimensions((1280, 720).into())
        },
    )
    .unwrap_or_log("InitGame");

    game.run();
}
