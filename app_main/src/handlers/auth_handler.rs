use crate::result::{result, result_data, result_error_msg};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, get, post, web};
use biz_service::biz_service::permission_service::PermissionService;
use biz_service::biz_service::role_permission_service::RolePermissionService;
use biz_service::biz_service::user_role_service::UserRoleService;
use biz_service::biz_service::user_service::UserService;
use common::errors::AppError;
use common::repository_util::Repository;
use mongo_macro::QueryFilter;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::option::Option;
use utoipa::ToSchema;
use validator::Validate;
use biz_service::biz_service::menu_service::MenuService;
use biz_service::entitys::menu_entity::MenuEntity;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_login);
    cfg.service(auth_logout);
    cfg.service(user_info);
    cfg.service(menu_list);
    cfg.service(permission_codes_handler);
}

#[derive(QueryFilter, Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginInfoDto {
    #[query(eq)]
    #[validate(length(min = 5, message = "name.too.short"))]
    pub user_name: Option<String>,
    #[validate(length(min = 5, message = "password.too.short"))]
    pub password: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginSessionDto {
    pub token: String,
    pub user_name: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    login_status: bool,
    token: String,
}


#[utoipa::path(
    post,
    path = "/auth/logout",
    request_body = LoginInfoDto,
    responses(
        (status = 200, description = "Hello response", body = String)
    )
)]
#[post("/auth/logout")]
async fn auth_logout(dto: web::Json<LoginSessionDto>) -> Result<impl Responder, AppError> {
    let user_service = UserService::get();
    user_service.login_out(&dto.user_name).await?;
    Ok(result())
}

/// 登录请求DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    user_name: String,
    password: String,
}

/// 登录响应DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    token: String,
    permission_codes: Vec<String>,
    roles: Vec<String>,
}

/// 用户信息响应DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct UserInfoResponse {
    user_name: String,
    roles: Vec<String>,
    // 可以根据需要增加其它用户信息字段，例如昵称、头像等
}

/// 菜单节点响应DTO（用于构建菜单树）
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct MenuNode {
    id: String,
    title: String,
    path: String,
    icon: Option<String>,
    component: Option<String>,
    hidden: bool,
    children: Vec<MenuNode>,
}

// 定义Claims结构
#[derive(Serialize, Deserialize)]
struct JwtClaims {
    sub: String, // 用户ID
    exp: usize,  // 过期时间戳
                 // 其他需要放入JWT的信息，例如角色、权限等
}
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "用户名或密码错误"),
        (status = 500, description = "登录过程异常")
    )
)]
// 登录处理函数
#[post("/auth/login")]
async fn user_login(req: web::Json<LoginRequest>) -> impl Responder {
    let username = &req.user_name;
    let password = &req.password;
    // 调用 UserService 验证用户
    match UserService::get().authenticate(&username.clone(), password).await {
        Ok(Some((token,user))) => {
            // 用户验证成功，生成 JWT token
          let user_service = UserService::get();
            // 获取用户角色列表（用角色编码或名称表示）
            let roles = UserRoleService::get()
                .get_roles_by_user(&user.id)
                .await
                .unwrap_or_default();
            let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();

            // 获取用户的所有权限码列表
            let perm_codes = {
                if user.is_admin {
                    // 如果是超级管理员，拥有所有权限
                    PermissionService::get()
                        .get_all_permissions()
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|p| p.enabled)
                        .map(|p| p.code)
                        .collect()
                } else {
                    // 非管理员，根据角色获取权限
                    let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();
                    let perm_ids = RolePermissionService::get()
                        .get_permission_ids_by_roles(&role_ids)
                        .await
                        .unwrap_or_default();
                    if perm_ids.is_empty() {
                        Vec::new()
                    } else {
                        // 查询权限集合获取code
                        PermissionService::get()
                            .dao
                            .query(doc! { "id": { "$in": &perm_ids } })
                            .await
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|p| p.enabled)
                            .map(|p| p.code)
                            .collect()
                    }
                }
            };

            // 构造响应
            let resp = LoginResponse {
                token,
                permission_codes: perm_codes,
                roles: role_codes,
            };
            HttpResponse::Ok().json(resp)
        }
        Ok(None) => {
            // 用户名或密码错误 或 用户被禁用
            HttpResponse::Unauthorized().body("Invalid username or password")
        }
        Err(e) => {
            // 处理查询错误
            HttpResponse::InternalServerError().body(format!("Login error: {}", e))
        }
    }
}
#[utoipa::path(
    get,
    path = "/auth/user/info",
    responses(
        (status = 200, description = "当前用户信息", body = UserInfoResponse),
        (status = 401, description = "未认证或用户不存在")
    )
)]
// 用户信息处理函数
#[get("/auth/user/info")]
async fn user_info(req: HttpRequest) -> impl Responder {
    // 从请求的认证信息中提取用户ID（具体提取方式视JWT中间件而定，这里假设已解析好）
    if let Some(user_id) = req.extensions().get::<String>() {
        // 查询用户基本信息
        if let Ok(Some(user)) = UserService::get()
            .dao
            .query(doc! {"id": user_id})
            .await
            .map(|mut u| u.pop())
        {
            // 查询用户角色列表
            let roles = UserRoleService::get()
                .get_roles_by_user(user_id)
                .await
                .unwrap_or_default();
            let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();
            let info = UserInfoResponse {
                user_name: user.user_name,
                roles: role_codes,
            };
            return HttpResponse::Ok().json(info);
        }
    }
    HttpResponse::Unauthorized().body("Invalid token or user not found")
}

#[utoipa::path(
    get,
    path = "/auth/menus",
    responses(
        (status = 200, description = "用户菜单树结构", body = [MenuNode]),
        (status = 401, description = "未认证用户")
    )
)]
#[get("/auth/menus")]
async fn menu_list(req: HttpRequest) -> impl Responder {
    // 假设通过中间件拿到 user_id
    if let Some(user_id) = req.extensions().get::<String>() {
        // 获取所有菜单（已按order排序）
        let menus = MenuService::get().get_all_menus().await.unwrap_or_default();
        // 获取用户的权限码列表（可与登录相同逻辑）
        let perm_codes = {
            // 若需要先获取用户角色->权限，可以重用Login时的逻辑:
            let roles = UserRoleService::get()
                .get_roles_by_user(user_id)
                .await
                .unwrap_or_default();
            let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();
            let perm_ids = RolePermissionService::get()
                .get_permission_ids_by_roles(&role_ids)
                .await
                .unwrap_or_default();
            let codes = if !perm_ids.is_empty() {
                PermissionService::get()
                    .dao
                    .query(doc! { "id": { "$in": &perm_ids } })
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|p| p.enabled)
                    .map(|p| p.code)
                    .collect()
            } else {
                Vec::new()
            };
            // 若是管理员，可选择不过滤菜单（或者授予所有权限码）
            codes
        };

        // 过滤菜单：只有当菜单没有设置permission，或者设置了且用户拥有该权限，才保留
        let accessible_menus: Vec<MenuEntity> = menus
            .into_iter()
            .filter(|menu| {
                if let Some(perm) = &menu.permission {
                    perm_codes.contains(perm)
                } else {
                    true // 无需权限控制的菜单
                }
            })
            .collect();

        // 构建菜单树
        let menu_tree = build_menu_tree(&accessible_menus);
        return HttpResponse::Ok().json(menu_tree);
    }
    HttpResponse::Unauthorized().finish()
}


// 辅助函数：将平铺的菜单列表构建为树结构
fn build_menu_tree(menus: &[MenuEntity]) -> Vec<MenuNode> {
    // 把菜单按照 parent_id 分组
    let mut menu_map: std::collections::HashMap<Option<String>, Vec<MenuEntity>> =
        std::collections::HashMap::new();
    for menu in menus {
        menu_map
            .entry(menu.parent_id.clone())
            .or_default()
            .push(menu.clone());
    }
    // 确保每个分组内部按照 order 排序
    for (_pid, list) in menu_map.iter_mut() {
        list.sort_by(|a, b| a.order.cmp(&b.order));
    }
    // 递归构造节点
    fn build_nodes(
        parent_id: Option<String>,
        map: &std::collections::HashMap<Option<String>, Vec<MenuEntity>>,
    ) -> Vec<MenuNode> {
        let mut nodes = Vec::new();
        if let Some(children) = map.get(&parent_id) {
            for menu in children {
                let node = MenuNode {
                    id: menu.id.clone(),
                    title: menu.title.clone(),
                    path: menu.path.clone(),
                    icon: menu.icon.clone(),
                    component: menu.component.clone(),
                    hidden: menu.hidden,
                    children: build_nodes(Some(menu.id.clone()), map),
                };
                nodes.push(node);
            }
        }
        nodes
    }
    // 根节点的 parent_id 通常为 None 或空字符串，这里使用 None 来表示根
    build_nodes(None, &menu_map)
}
#[utoipa::path(
    get,
    path = "/auth/permissions",
    responses(
        (status = 200, description = "当前用户权限码列表", body = [String]),
        (status = 401, description = "未认证用户")
    )
)]
#[get("/auth/permissions")]
async fn permission_codes_handler(req: HttpRequest) -> impl Responder {
    if let Some(user_id) = req.extensions().get::<String>() {
        // 获取用户所有权限码（逻辑与上面菜单接口中的perm_codes计算类似）
        let roles = UserRoleService::get()
            .get_roles_by_user(user_id)
            .await
            .unwrap_or_default();
        let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();
        let perm_ids = RolePermissionService::get()
            .get_permission_ids_by_roles(&role_ids)
            .await
            .unwrap_or_default();
        let perm_codes: Vec<String> = if perm_ids.is_empty() {
            Vec::new()
        } else {
            PermissionService::get()
                .dao
                .query(doc! { "id": { "$in": &perm_ids } })
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|p| p.enabled)
                .map(|p| p.code)
                .collect()
        };
        return HttpResponse::Ok().json(perm_codes);
    }
    HttpResponse::Unauthorized().finish()
}
