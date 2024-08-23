use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use actix_files as fs;
use serde::Serialize;
use tera::Tera;
use walkdir::WalkDir;

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

async fn index(tmpl: web::Data<Tera>) -> impl Responder {
    let projects = get_projects();
    let mut ctx = tera::Context::new();
    ctx.insert("projects", &projects);
    
    let rendered = tmpl.render("index.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

async fn project(tmpl: web::Data<Tera>, path: web::Path<String>) -> impl Responder {
    let project_name = path.into_inner();
    let project_images = get_project_images(&project_name);
    let mut ctx = tera::Context::new();
    ctx.insert("project", &project_images);
    
    let rendered = tmpl.render("project.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(rendered)
}

fn get_projects() -> Vec<Project> {
    WalkDir::new("img")
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
                .map(|e| e.path().strip_prefix("img/").unwrap().to_string_lossy().into_owned())
                .unwrap_or_else(|| "placeholder.jpg".to_string());
            Project { name, preview_image }
        })
        .collect()
}

fn get_project_images(project_name: &str) -> ProjectImages {
    let images = WalkDir::new(format!("img/{}", project_name))
        .max_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file() && 
            entry.path().extension().map_or(false, |ext| 
                ext == "jpg" || ext == "png" || ext == "gif"
            )
        })
        .map(|entry| entry.path().strip_prefix("img/").unwrap().to_string_lossy().into_owned())
        .collect();
    
    ProjectImages {
        name: project_name.to_string(),
        images,
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let tera = Tera::new("templates/**/*").unwrap();
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tera.clone()))
            .service(fs::Files::new("/img", "img").show_files_listing())
            .service(web::resource("/").to(index))
            .service(web::resource("/project/{name}").to(project))
    })
    .bind("0.0.0.0:3030")?
    .run()
    .await
}
