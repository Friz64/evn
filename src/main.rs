use evn_engine::prelude::*;

fn main() {
    let game = GameBuilder::new("Indev")
        .unwrap()
        .with_config("config.yml", include_resource!(open: "config.yml"))
        .unwrap_or_log("Config Error")
        .with_shader(
            "normal",
            include_resource!(closed: "shaders/normal.vert.spv"),
            include_resource!(closed: "shaders/normal.frag.spv"),
        )
        .build();

    game.run();
}
