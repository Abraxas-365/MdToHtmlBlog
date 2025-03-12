use crate::apierror::ApiError;
use crate::renderer::Renderer;
use actix_web::{get, web, HttpResponse, Scope};

#[get("/{path:.*}")]
async fn render_blog_page(
    path: web::Path<String>,
    renderer: web::Data<Renderer>,
) -> Result<HttpResponse, ApiError> {
    let path_str = path.into_inner();

    // Render the markdown
    let html = renderer.render(&path_str)?;

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub fn blog_routes() -> Scope {
    web::scope("/blog").service(render_blog_page)
}
