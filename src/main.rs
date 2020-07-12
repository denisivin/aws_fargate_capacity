use chrono::{TimeZone, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use rusoto_core::Region;
use rusoto_ecs::{Ecs, EcsClient, ListServicesRequest, DescribeServicesRequest, DescribeTaskDefinitionRequest};

#[tokio::main]
async fn main() {
    const CLUSTER_NAME: &str = "temforce-cluster";
    let client = EcsClient::new(Region::UsWest2);

    let mut page = 0;
    let page_size = 10;
    let mut result: Vec<(String, f32, f32, String, i64, String, String)> = Vec::new();

    let list_services_req = ListServicesRequest {
        cluster: Some(String::from(CLUSTER_NAME)),
        max_results: Some(100),
        ..Default::default()
    };

    println!("Fetching services...");

    match client.list_services(list_services_req).await {
        Ok(res) => match res.service_arns {
            Some(mut service_list) => {
                service_list.sort();

                let pb = ProgressBar::new(service_list.len() as u64);
                pb.set_style(ProgressStyle::default_bar()
                  .template("{spinner:.green} [{elapsed_precise}] [{bar:60.cyan/blue}] {pos:>3}/{len:3} ({eta})")
                  .progress_chars("##-"));

                loop {
                    let start = if service_list.len() < page { break } else { page };

                    let end = if service_list.len() >= start + page_size
                              { start + page_size } else { service_list.len() };

                    let slice = &service_list[start..end];

                    let describe_services_req = DescribeServicesRequest {
                        cluster: Some(String::from(CLUSTER_NAME)),
                        services: slice.to_vec(),
                        include: None
                    };

                    match client.describe_services(describe_services_req).await {
                        Ok(res) => match res.services {
                            Some(service_list) => {

                                for service in service_list {

                                    let describe_taskdef_req = DescribeTaskDefinitionRequest {
                                        task_definition: service.task_definition.unwrap_or_default(),
                                        include: None
                                    };

                                    match client.describe_task_definition(describe_taskdef_req).await {
                                        Ok(res) => {
                                            let task_def = res.task_definition.unwrap_or_default();

                                            result.push((
                                                service.service_name.unwrap_or_default(),
                                                task_def.cpu.unwrap_or_default().parse::<f32>().unwrap_or_default() / 1024.0,
                                                task_def.memory.unwrap_or_default().parse::<f32>().unwrap_or_default() / 1024.0,
                                                service.status.unwrap_or_default(),
                                                service.running_count.unwrap_or_default(),
                                                service.platform_version.unwrap_or_default(),
                                                match service.created_at {
                                                    Some(v) => Utc.timestamp(v as i64, 0).to_rfc3339(),
                                                    None => String::from("n/a")
                                                })
                                            );

                                            pb.inc(1);
                                        },
                                        Err(_) => continue
                                    }
                                }
                            }
                            None => continue
                        },
                        Err(_) => continue
                    }

                    page += page_size;
                }

                pb.finish_and_clear();
                println!("{} services found in cluster {}:", result.len(), CLUSTER_NAME);
                println!("{:<30} {: <10} {: <10} {: <10} {: <7} {: <10} {: <10}",
                    "Name", "vCPU", "RAM (GB)", "Status", "Count", "Version", "Created"
                );

                for e in result.iter() {
                    println!("{:<30} {: <10} {: <10} {: <10} {: <7} {: <10} {: <10}",
                             e.0, e.1, e.2, e.3, e.4, e.5, e.6);
                }
            }
            None => println!("No services found!")
        },
        Err(error) => {
            eprintln!("Error: {:?}", error);
        }
    }
}
