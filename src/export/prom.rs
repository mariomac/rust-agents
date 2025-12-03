use std::collections::HashMap;
use std::fmt::{Debug, Display, Error, Formatter};
use std::sync::{Arc, Mutex};
use crate::tracers::datapoint;
use rouille::router;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};
use crate::tracers::datapoint::Resource;

pub async fn metrics_exporter(ct: CancellationToken, mut input: mpsc::Receiver<Vec<datapoint::Resource>>) {
    let mut registry = Arc::new(Mutex::new(PromRegistry::default()));
    let reg_reader = registry.clone();
    let addr = "0.0.0.0:9090".to_string();

    tokio::spawn(async move {
        rouille::start_server(addr, move |request| {
            let reg = reg_reader.lock().unwrap();
            router!(request,
            (GET) (/metrics) => {
                    //  TODO: do something more efficient
                rouille::Response::text(reg.metrics())
            },
            // The code block is called if none of the other blocks matches the request.
            // We return an empty response with a 404 status code.
            _ => rouille::Response::empty_404()
        )
        });
    });

    loop {
        tokio::select!(
            _ = ct.cancelled() => {
                debug!("process tracer cancelled");
                return
            },
            Some(resources) = input.recv() => {
                println!("received {} resources", resources.len());
                let mut reg = registry.lock().unwrap();
                for resource in &resources {
                    reg.register(resource);
                }
            },
        )
    }
}

struct PromRegistry {
    metrics: HashMap<String, Counter>,
}

struct Counter {
    name: String,
    instances: HashMap<LabelVals, u64>,
}

type LabelVals = Vec<(String, String)>;

impl PromRegistry {
    fn default() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    pub fn register(&mut self, res: &Resource) {
        for metric in &res.metrics {
            let reg_counter = self.metrics.entry(metric.name.clone()).or_insert(Counter {
                name: metric.name.clone(),
                instances: HashMap::new(),
            });
            let mut reg_attrs = res.attrs.clone();
            reg_attrs.push(("instance".to_string(), res.instance.clone()));
            reg_attrs.extend(metric.attrs.clone());

            let mut reg_val = reg_counter.instances.entry(reg_attrs).or_insert(0);
            *reg_val += metric.value;
        }
    }

    fn metrics(&self) -> String {
        let mut strb = String::new();
        self.metrics.iter().for_each( |(name, counter)| {
            counter.instances.iter().for_each(|(lbls, count)| {
                strb.push_str(name);
                if lbls.len() > 0 {
                    strb.push_str(r#"{""#);
                    lbls.iter().for_each(|(lbl, val)| {
                        strb.push_str(lbl);
                        strb.push_str(r#""=""#);
                        strb.push_str(val);
                        strb.push_str(r#"",""#);
                    });
                    strb.pop();
                    strb.pop();
                    strb.push_str("} ");
                    strb.push_str(count.to_string().as_str());
                    strb.push('\n');
                }
            })
        });
        strb
    }
}


