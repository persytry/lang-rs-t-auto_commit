//! @author persy
//! @date 2021/11/11 13:40

use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::fs;
use clap::{Arg, App};
extern crate serde_derive;
extern crate serde_json;
extern crate chrono;
use chrono::offset::Local;
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct CfgItem{
    need_commit: bool,
    dir: String,
}

#[derive(Serialize, Deserialize)]
struct Cfg{
    delta_seconds: u16,
    include: Vec<CfgItem>,
    exclude: Vec<String>,
}

impl Cfg{
    fn new() -> Self{
        Cfg{
            delta_seconds: 60 * 60 * 13,
            include: vec![
                CfgItem{ need_commit: false, dir: "/root/repo".to_string() },
                CfgItem{ need_commit: true, dir: "/root/a/git/lang/py/finance/asset".to_string() }
            ],
            exclude: vec!["/root".to_string()],
        }
    }

    fn commit_push_one_proj(&self, need_commit: bool, dir: &String) -> u32{
        for d in self.exclude.iter(){
            if d == dir{
                return 0;
            }
        }
        println!("{},begin commit or push the repo: {}. -----begin-----", Local::now(), dir);
        if need_commit{
            exec(&dir, "git", &["commit","-a","-m","\"auto commit by proc\""]);
        }
        exec(&dir, "git", &["push"]);
        println!("{},----------the repo commit or push end-----------", Local::now());
        1
    }

    fn recursion_commit(&self, need_commit: bool, dir: &String) -> u32{
        if let Some(idx) = dir.find(".git"){
            if idx > 0{
                return self.commit_push_one_proj(need_commit, &dir);
            }
        }
        let mut dirs = Vec::new();
        for entry in fs::read_dir(dir).expect(&format!("can not read dir:{}", dir)){
            let entry = entry.unwrap();
            let path;
            if entry.file_type().unwrap().is_symlink(){
                path = fs::read_link(&entry.path()).unwrap();
            }
            else{
                path = entry.path();
            }
            if path.is_dir(){
                if ".git" == entry.file_name().to_str().unwrap(){
                    return self.commit_push_one_proj(need_commit, &dir);
                }
                dirs.push(path.to_str().unwrap().to_string());
            }
        }
        let mut cnt = 0;
        for dir in dirs{
            cnt += self.recursion_commit(need_commit, &dir);
        }
        cnt
    }
}

fn parse_args() -> Option<String>{
    let matches = App::new("auto_commit")
                          .version("0.1.0")
                          .author("persy")
                          .about("auto commit and push git")
                          .arg(Arg::with_name("cfg")
                               .short("c")
                               .long("cfg")
                               .value_name("FILE")
                               .help("set json config path")
                               .takes_value(true))
                          .arg(Arg::with_name("service")
                               .short("s")
                               .long("service")
                               .help("set enable service when restart system")
                               .takes_value(false))
                          .arg(Arg::with_name("json")
                               .short("j")
                               .long("json")
                               .help("generate default json cfg")
                               .takes_value(false))
                          .get_matches();
    let cfg = matches.value_of("cfg").unwrap_or("/etc/auto_commit.json");
    if matches.occurrences_of("service") > 0{
        gen_service_cfg("/lib/systemd/system/auto_commit.service", &cfg.to_string());
        return None;
    }
    if matches.occurrences_of("json") > 0{
        let v = serde_json::to_string_pretty(&Cfg::new()).unwrap();
        fs::write(cfg.to_string(), &v).unwrap();
        return None;
    }
    Some(cfg.to_string())
}

fn gen_service_cfg(service_path: &str, cfg_path: &String){
    let s = format!(r"[Unit]
Description=gen auto_commit service
After=network.target

[Service]
User=root
ExecStart=auto_commit -c {}

[Install]
WantedBy=multi-user.target", cfg_path);
    fs::write(service_path, s).unwrap();
    Command::new("systemctl").arg("daemon-reload").status().ok();
    let path = Path::new(&service_path);
    Command::new("systemctl").arg("enable").arg(path.file_name().unwrap().to_str().unwrap()).status().ok();
}

fn run(cfg: &Cfg) -> !{
    loop{
        let mut cnt: u32 = 0;
        for item in cfg.include.iter(){
            cnt += cfg.recursion_commit(item.need_commit, &item.dir);
        }
        println!("commit or push done, total count: {}", cnt);
        sleep(Duration::from_secs(cfg.delta_seconds as u64));
    }
}

fn exec(dir: &String, cmd: &str, args: &[&str]){
    if let Ok(mut child) = Command::new(cmd).current_dir(dir).args(args).spawn(){
        for _ in 0..100{
            match child.try_wait(){
                Ok(Some(_)) => break,
                Ok(None) => sleep(Duration::from_secs(1)),
                Err(e) =>{
                    println!("{},exec command error: {}", Local::now(), e);
                    break;
                }
            }
        }
        child.kill().ok();
    }
}

fn main() {
    if let Some(cfg_path) = parse_args(){
        let cfg: Cfg = serde_json::from_str(&fs::read_to_string(&cfg_path).expect(&format!("config file open failed:{}", &cfg_path))).unwrap();
        run(&cfg);
    }
}
