use porumbel::config::{self, Cmd, Config, OpenMode};
use porumbel::hub::Hub;
use porumbel::{adb, launch, qr, server, util, watch};
use std::sync::Arc;
use std::time::Duration;

fn fail(msg: &str) -> ! {
    eprintln!("porumbel: {msg}");
    std::process::exit(2);
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == name {
            return it.next().cloned();
        }
        if let Some(v) = a.strip_prefix(&format!("{name}=")) {
            return Some(v.to_string());
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = config::config_path(flag_value(&args, "--config").as_deref());
    let mut cfg = Config::default();

    // quick exits that need no settings file
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{}", config::HELP);
        return;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("porumbel {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    match config::ensure_file(&path) {
        Ok(true) => eprintln!("porumbel: created settings file {}", path.display()),
        Ok(false) => {}
        Err(e) => fail(&e),
    }
    if let Err(e) = cfg.load_file(&path) {
        fail(&e);
    }
    let cmd = match config::apply_args(&mut cfg, &args) {
        Ok(c) => c,
        Err(e) => fail(&e),
    };
    if cfg.token.is_empty() {
        fail(&format!(
            "no token in {} — add a line `token = something-long`",
            path.display()
        ));
    }

    match cmd {
        Cmd::Help => {
            print!("{}", config::HELP);
            return;
        }
        Cmd::Version => {
            println!("porumbel {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Cmd::Init => {
            println!("{}", path.display());
            return;
        }
        Cmd::PrintConfig => {
            let mut shown = cfg.clone();
            shown.token = format!("{}…", shown.token.chars().take(4).collect::<String>());
            print!("# {}\n{}", path.display(), shown.render());
            return;
        }
        _ => {}
    }

    let hub = match Hub::new(cfg.clone()) {
        Ok(h) => Arc::new(h),
        Err(e) => fail(&format!("cannot prepare {}: {e}", cfg.inbox.display())),
    };

    match cmd {
        Cmd::Forget => {
            hub.forget();
            return;
        }
        Cmd::OpenLast => {
            match hub.latest_session() {
                Some(n) => match porumbel::open_named(&hub, &n) {
                    Ok(_) => {}
                    Err(e) => fail(&e),
                },
                None => fail("no session with photos yet"),
            }
            std::thread::sleep(Duration::from_millis(300));
            return;
        }
        Cmd::Doctor => {
            let bad = porumbel::doctor::run(&hub, &path);
            std::process::exit(if bad > 0 { 1 } else { 0 });
        }
        Cmd::Qr => {
            match hub.phone_url() {
                Some(u) => {
                    match qr::Qr::encode(u.as_bytes()) {
                        Ok(q) => print!("{}", q.to_terminal()),
                        Err(e) => fail(&e),
                    }
                    println!("{u}");
                }
                None => fail("no local network address found"),
            }
            return;
        }
        _ => {}
    }

    if cfg.serve && cfg.token.len() < 16 {
        eprintln!("porumbel: warning: the token is short ({} characters); anyone on your network could guess it. Use 16 or more.", cfg.token.len());
    }
    banner(&hub, &path);

    let mut threads = Vec::new();
    if cfg.serve {
        match server::start(hub.clone()) {
            Ok(addr) => hub.log(format!("server listening on {addr}")),
            Err(e) => fail(&format!("cannot listen on {}:{}: {e}", cfg.bind, cfg.port)),
        }
    }
    if cfg.adb {
        let h = hub.clone();
        threads.push(std::thread::spawn(move || adb::run_loop(&h)));
    }
    for d in cfg.watch.clone() {
        let h = hub.clone();
        threads.push(std::thread::spawn(move || watch::run_loop(&h, d)));
    }

    if cfg.once {
        for t in threads {
            let _ = t.join();
        }
        // hand over whatever arrived
        if let Some(n) = hub.latest_session() {
            hub.close_current();
            if cfg.open == OpenMode::Auto {
                let _ = porumbel::open_named(&hub, &n);
                std::thread::sleep(Duration::from_millis(300));
            }
        }
        return;
    }
    if !cfg.serve && !cfg.adb && cfg.watch.is_empty() {
        fail("nothing to do: enable the server, --adb or --watch DIR");
    }

    // main loop: close idle sessions and hand them to LegatorOCR
    loop {
        std::thread::sleep(Duration::from_secs(1));
        if let Some(name) = hub.tick() {
            if let Err(e) = porumbel::open_named(&hub, &name) {
                eprintln!("porumbel: {e}");
            }
        }
    }
}

fn banner(hub: &Hub, conf: &std::path::Path) {
    let cfg = &hub.cfg;
    eprintln!("PORUMBEL {}", env!("CARGO_PKG_VERSION"));
    eprintln!("  settings : {}", conf.display());
    eprintln!("  inbox    : {}", cfg.inbox.display());
    let lg = launch::describe(cfg);
    eprintln!(
        "  LegatorOCR: {}",
        if lg.is_empty() {
            "not found (the folder will be opened instead; set `launch =`)".to_string()
        } else {
            lg
        }
    );
    if cfg.serve {
        match hub.lan.first() {
            Some(ip) => {
                eprintln!(
                    "  phone    : http://{ip}:{}/?t=…   (run `porumbel qr` for the QR code)",
                    cfg.port
                );
                eprintln!(
                    "  WebDAV   : http://{ip}:{}/dav/   password = token",
                    cfg.port
                );
            }
            None => eprintln!(
                "  server   : port {}, but no local network address was found",
                cfg.port
            ),
        }
        eprintln!(
            "  panel    : http://localhost:{}/?t={}",
            cfg.port, cfg.token
        );
        if cfg.bind == "0.0.0.0" {
            eprintln!(
                "  note     : the server is reachable from your network; keep the token private"
            );
        }
    }
    let _ = util::now_unix();
}
