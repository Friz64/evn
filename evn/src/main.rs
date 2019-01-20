use evn_engine::prelude::*;

fn main() {
    let game = Game::new(
        "Indev",
        |res_builder| {
            res_builder
                .with_config("config.yml", include_resource!(open: "config.yml"))
                .unwrap_or_log("Config")
                .with_shader(
                    "normal",
                    include_resource!(closed: "shaders/normal.vert.spv"),
                    include_resource!(closed: "shaders/normal.frag.spv"),
                )
        },
        |window_builder| {
            window_builder
                .with_title("evn")
                .with_dimensions((1280, 720).into())
        },
    )
    .unwrap();

    game.run();
}
