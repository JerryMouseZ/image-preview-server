use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files as fs;
use serde::Serialize;
use tera::Tera;
use walkdir::WalkDir;
use clap::Parser;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "img")]
    image_dir: String,

    #[arg(short, long, default_value_t = false, help = "Enable video files display")]
    video: bool,
}

#[derive(Serialize)]
struct Project {
    name: String,
    preview_image: String,
}

#[derive(Serialize)]
struct MediaFile {
    path: String,
    is_video: bool,
}

#[derive(Serialize)]
struct ProjectImages {
    name: String,
    media_files: Vec<MediaFile>,
}

struct AppState {
    image_dir: String,
    show_video: bool,
}

fn is_supported_image(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg" | "png" | "gif" | "webp")
}

fn is_supported_video(ext: &str) -> bool {
    matches!(ext, "mp4" | "webm" | "ogg" | "mov")
}

fn is_supported_media(ext: &str, show_video: bool) -> bool {
    is_supported_image(ext) || (show_video && is_supported_video(ext))
}

fn is_media_file(entry: walkdir::DirEntry, show_video: bool) -> bool {
    entry.file_type().is_file() && 
    entry.path().extension()
        .map_or(false, |ext| is_supported_media(ext.to_string_lossy().to_lowercase().as_str(), show_video))
}

async fn index(tmpl: web::Data<Tera>, data: web::Data<AppState>) -> impl Responder {
    let projects = get_projects(&data.image_dir, data.show_video);
    let mut ctx = tera::Context::new();
    ctx.insert("projects", &projects);
    
    let rendered = tmpl.render("index.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(rendered)
}

async fn project(
    tmpl: web::Data<Tera>, 
    path: web::Path<String>,
    data: web::Data<AppState>
) -> impl Responder {
    let encoded_project_name = path.into_inner();
    let project_name = percent_decode_str(&encoded_project_name)
        .decode_utf8_lossy()
        .into_owned();
    
    println!("Project name: {}", project_name);
    let project_images = get_project_images(&data.image_dir, &project_name, data.show_video);
    let mut ctx = tera::Context::new();
    ctx.insert("project", &project_images);
    
    let rendered = tmpl.render("project.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(rendered)
}

fn get_projects(image_dir: &str, show_video: bool) -> Vec<Project> {
    let mut projects = Vec::new();
    let base_path = Path::new(image_dir);

    // 检查根目录是否直接包含媒体文件
    let has_root_media = WalkDir::new(image_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|e| is_media_file(e, show_video));

    // 如果根目录直接包含媒体文件，添加一个"root"项目
    if has_root_media {
        let preview_image = WalkDir::new(image_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && 
                e.path().extension()
                    .map_or(false, |ext| is_supported_image(ext.to_string_lossy().to_lowercase().as_str()))
            })
            .min_by_key(|e| e.file_name().to_string_lossy().into_owned())
            .map(|e| {
                let rel_path = e.path().strip_prefix(image_dir).unwrap();
                rel_path.to_string_lossy().into_owned()
            })
            .unwrap_or_else(|| "placeholder.jpg".to_string());

        projects.push(Project {
            name: "root".to_string(),
            preview_image,
        });
    }

    // 递归遍历所有子目录
    for entry in WalkDir::new(image_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_dir())
    {
        let dir_path = entry.path();
        
        // 检查目录是否包含媒体文件
        let has_media = WalkDir::new(dir_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| is_media_file(e, show_video));

        if has_media {
            let rel_path = dir_path.strip_prefix(base_path).unwrap();
            let name = rel_path.to_string_lossy().into_owned();
            
            let preview_image = WalkDir::new(dir_path)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().is_file() && 
                    e.path().extension()
                        .map_or(false, |ext| is_supported_image(ext.to_string_lossy().to_lowercase().as_str()))
                })
                .min_by_key(|e| e.file_name().to_string_lossy().into_owned())
                .map(|e| {
                    let rel_path = e.path().strip_prefix(image_dir).unwrap();
                    rel_path.to_string_lossy().into_owned()
                })
                .unwrap_or_else(|| "placeholder.jpg".to_string());

            projects.push(Project { name, preview_image });
        }
    }
    
    // 按名称排序
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects
}

fn get_project_images(image_dir: &str, project_name: &str, show_video: bool) -> ProjectImages {
    let project_path = if project_name == "root" {
        Path::new(image_dir).to_path_buf()
    } else {
        Path::new(image_dir).join(project_name)
    };
    
    let mut media_files: Vec<MediaFile> = if project_name == "root" {
        // 对于根目录，只获取直接位于根目录的媒体文件
        WalkDir::new(&project_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| is_media_file(entry.clone(), show_video))
            .filter_map(|entry| {
                entry.path()
                    .strip_prefix(image_dir)
                    .ok()
                    .map(|path| {
                        let path_str = path.to_string_lossy().into_owned();
                        let is_video = entry.path()
                            .extension()
                            .map_or(false, |ext| is_supported_video(ext.to_string_lossy().to_lowercase().as_str()));
                        MediaFile {
                            path: path_str,
                            is_video,
                        }
                    })
            })
            .collect()
    } else {
        // 对于子目录，递归获取该目录下的所有媒体文件
        WalkDir::new(&project_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| is_media_file(entry.clone(), show_video))
            .filter_map(|entry| {
                entry.path()
                    .strip_prefix(image_dir)
                    .ok()
                    .map(|path| {
                        let path_str = path.to_string_lossy().into_owned();
                        let is_video = entry.path()
                            .extension()
                            .map_or(false, |ext| is_supported_video(ext.to_string_lossy().to_lowercase().as_str()));
                        MediaFile {
                            path: path_str,
                            is_video,
                        }
                    })
            })
            .collect()
    };
    
    // 按名称排序
    media_files.sort_by(|a, b| a.path.cmp(&b.path));
    
    ProjectImages {
        name: project_name.to_string(),
        media_files,
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let image_dir = args.image_dir;
    let show_video = args.video;
    
    println!("Using image directory: {}", image_dir);
    println!("Video display: {}", if show_video { "enabled" } else { "disabled" });
    
    let mut tera = Tera::new("templates/**/*").unwrap();
    
    // 添加自定义过滤器用于严格的URL编码
    tera.register_filter("urlencode_strict", |value: &tera::Value, _: &std::collections::HashMap<String, tera::Value>| {
        if let tera::Value::String(s) = value {
            Ok(tera::Value::String(utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()))
        } else {
            Ok(tera::Value::Null)
        }
    });
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(AppState {
                image_dir: image_dir.clone(),
                show_video,
            }))
            .service(
                fs::Files::new("/img", &image_dir)
                    .show_files_listing()
                    .use_hidden_files()
                    .prefer_utf8(true)
            )
            .service(web::resource("/").to(index))
            .service(web::resource("/project/{name}").to(project))
    })
    .bind("0.0.0.0:3030")?
    .run()
    .await
}
