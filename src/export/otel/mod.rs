use crate::tracers::proc::Process;
use futures::SinkExt;
use opentelemetry::KeyValue;
use opentelemetry::global::ObjectSafeTracer;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_otlp::{
    HttpExporterBuilder, MetricExporter, Protocol, WithExportConfig, WithHttpConfig,
};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn init_otel_resource() -> opentelemetry_sdk::Resource {
    let otlp_resource_detected = opentelemetry_sdk::Resource::builder()
        .with_detector(Box::new(
            opentelemetry_sdk::resource::SdkProvidedResourceDetector,
        ))
        .with_detector(Box::new(
            opentelemetry_sdk::resource::EnvResourceDetector::new(),
        ))
        .with_detector(Box::new(
            opentelemetry_sdk::resource::TelemetryResourceDetector,
        ))
        .with_service_name("rust-agents-test");

    otlp_resource_detected.build()
}

// ************************************ METRICS ************************************
fn init_metrics(
    // config: &Config,
    resource: opentelemetry_sdk::Resource,
) -> opentelemetry_sdk::metrics::SdkMeterProvider {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());
    let headers = std::env::var("OTEL_EXPORTER_OTLP_HEADERS")
        .map(|h| parse_headers(&h))
        .unwrap_or_default();
    // let protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL")
    //     .unwrap_or_else(|_| "http/protobuf".to_string());

    println!("OTEL_EXPORTER_OTLP_ENDPOINT: {}", endpoint);
    println!("OTEL_EXPORTER_OTLP_HEADERS: {:?}", headers);

    let mut meter_provider_builder = opentelemetry_sdk::metrics::SdkMeterProvider::builder();

    // if config.std_stream_metrics_exporter_enabled {
    //     let std_stream_exporter = opentelemetry_stdout::MetricExporter::default();
    //     let std_stream_reader =
    //         opentelemetry_sdk::metrics::PeriodicReader::builder(std_stream_exporter)
    //             .with_interval(Duration::from_secs(10))
    //             .build();
    //
    //     meter_provider_builder = meter_provider_builder.with_reader(std_stream_reader);
    // }
    // if config.otel_collector_metrics_exporter_enabled {
        let otlp_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .with_headers(headers)
            .with_protocol(Protocol::HttpBinary)
            // .with_tonic()
            .build()
            .unwrap();

        let otlp_reader = opentelemetry_sdk::metrics::PeriodicReader::builder(otlp_exporter)
            .with_interval(Duration::from_secs(10))
            .build();

        meter_provider_builder = meter_provider_builder.with_reader(otlp_reader);
    // }

    let meter_provider = meter_provider_builder.with_resource(resource).build();
    meter_provider
}

fn parse_headers(header_str: &str) -> HashMap<String, String> {
    header_str
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.trim().split('=');
            Some((
                parts.next()?.to_string(),
                parts.next()?.to_string(),
            ))
        })
        .collect()
}


pub async fn exporter(ct: CancellationToken, mut input: tokio::sync::mpsc::Receiver<Vec<Process>>) {
    std::env::vars()
        .filter(|(key, _)| key.starts_with("OTEL_"))
        .for_each(|(key, _)| println!("{}", key));

    let resource = init_otel_resource();
    let provider = init_metrics(resource);

    let meter = provider.meter("metric_exporter");

    // TODO: observable counter to remove process metrics when they are no longer needed
    let proc_cpu_time = meter.u64_counter("process.cpu.time").build();

    loop {
        tokio::select!(
            _ = ct.cancelled() => {
                provider.shutdown();
                return;
            }
            Some(metrics) = input.recv() => {
                metrics.iter().take(3).for_each(|proc| {
                    proc_cpu_time.add(metrics.len() as u64, &[
                        // this IS NOT compliant with the OTEL standard, as the following attributes
                        // are resource attributes, not metric attributes
                        KeyValue::new("process.pid", proc.pid.to_string()),
                        KeyValue::new("process.executable.path", proc.exe.to_string()),
                        KeyValue::new("process.cpu.time", proc.cpu_time.to_string())
                    ]);
                });
                // if let Err(sdkErr) = provider.force_flush() {
                //     println!("Error flushing metrics: {:?}", sdkErr);
                // }
            },
        )
    }
}
