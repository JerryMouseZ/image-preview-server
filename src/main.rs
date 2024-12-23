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
}

#[derive(Serialize)]
struct Project {
    name: String,
    preview_image: String,
}

#[derive(Serialize)]
struct ProjectImages {
    name: String,
    images: Vec<String>,
}

struct AppState {
    image_dir: String,
}

async fn index(tmpl: web::Data<Tera>, data: web::Data<AppState>) -> impl Responder {
    let projects = get_projects(&data.image_dir);
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
    let project_images = get_project_images(&data.image_dir, &project_name);
    let mut ctx = tera::Context::new();
    ctx.insert("project", &project_images);
    
    let rendered = tmpl.render("project.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(rendered)
}

fn get_projects(image_dir: &str) -> Vec<Project> {
    let mut projects = Vec::new();
    let base_path = Path::new(image_dir);

    // 检查根目录是否直接包含图片文件
    let has_root_images = WalkDir::new(image_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|e| {
            e.file_type().is_file() && 
            e.path().extension().map_or(false, |ext| 
                ext == "jpg" || ext == "png" || ext == "gif"
            )
        });

    // 如果根目录直接包含图片，添加一个"root"项目
    if has_root_images {
        let preview_image = WalkDir::new(image_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && 
                e.path().extension().map_or(false, |ext| 
                    ext == "jpg" || ext == "png" || ext == "gif"
                )
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
        
        // 检查目录是否包含图片文件
        let has_images = WalkDir::new(dir_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| {
                e.file_type().is_file() && 
                e.path().extension().map_or(false, |ext| 
                    ext == "jpg" || ext == "png" || ext == "gif"
                )
            });

        if has_images {
            let rel_path = dir_path.strip_prefix(base_path).unwrap();
            let name = rel_path.to_string_lossy().into_owned();
            
            let preview_image = WalkDir::new(dir_path)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().is_file() && 
                    e.path().extension().map_or(false, |ext| 
                        ext == "jpg" || ext == "png" || ext == "gif"
                    )
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

fn get_project_images(image_dir: &str, project_name: &str) -> ProjectImages {
    let project_path = if project_name == "root" {
        Path::new(image_dir).to_path_buf()
    } else {
        Path::new(image_dir).join(project_name)
    };
    
    let mut images: Vec<String> = if project_name == "root" {
        // 对于根目录，只获取直接位于根目录的图片
        WalkDir::new(&project_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().is_file() && 
                entry.path().extension().map_or(false, |ext| 
                    ext == "jpg" || ext == "png" || ext == "gif"
                )
            })
            .filter_map(|entry| {
                entry.path()
                    .strip_prefix(image_dir)
                    .ok()
                    .map(|path| path.to_string_lossy().into_owned())
            })
            .collect()
    } else {
        // 对于子目录，递归获取该目录下的所有图片
        WalkDir::new(&project_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().is_file() && 
                entry.path().extension().map_or(false, |ext| 
                    ext == "jpg" || ext == "png" || ext == "gif"
                )
            })
            .filter_map(|entry| {
                entry.path()
                    .strip_prefix(image_dir)
                    .ok()
                    .map(|path| path.to_string_lossy().into_owned())
            })
            .collect()
    };
    
    // 按名称排序
    images.sort();
    
    ProjectImages {
        name: project_name.to_string(),
        images,
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let image_dir = args.image_dir;
    
    println!("Using image directory: {}", image_dir);
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
