use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files as fs;
use serde::Serialize;
use tera::Tera;
use walkdir::WalkDir;
use clap::Parser;

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
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

async fn project(
    tmpl: web::Data<Tera>, 
    path: web::Path<String>,
    data: web::Data<AppState>
) -> impl Responder {
    let project_name = path.into_inner();
    println!("Project name: {}", project_name);
    let project_images = get_project_images(&data.image_dir, &project_name);
    let mut ctx = tera::Context::new();
    ctx.insert("project", &project_images);
    
    let rendered = tmpl.render("project.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

fn get_projects(image_dir: &str) -> Vec<Project> {
    WalkDir::new(image_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let preview_image = WalkDir::new(entry.path())
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .find(|e| {
                    e.file_type().is_file() && 
                    e.path().extension().map_or(false, |ext| 
                        ext == "jpg" || ext == "png" || ext == "gif"
                    )
                })
                .map(|e| e.path().strip_prefix(image_dir).unwrap().to_string_lossy().into_owned())
                .unwrap_or_else(|| "placeholder.jpg".to_string());
            Project { name, preview_image }
        })
        .collect()
}

fn get_project_images(image_dir: &str, project_name: &str) -> ProjectImages {
    let images = WalkDir::new(format!("{}/{}", image_dir, project_name))
        .max_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file() && 
            entry.path().extension().map_or(false, |ext| 
                ext == "jpg" || ext == "png" || ext == "gif"
            )
        })
        .map(|entry| entry.path().strip_prefix(image_dir).unwrap().to_string_lossy().into_owned())
        .collect();
    
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
    let tera = Tera::new("templates/**/*").unwrap();
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(AppState {
                image_dir: image_dir.clone(),
            }))
            .service(fs::Files::new("/img", &image_dir).show_files_listing())
            .service(web::resource("/").to(index))
            .service(web::resource("/project/{name}").to(project))
    })
    .bind("0.0.0.0:3030")?
    .run()
    .await
}
