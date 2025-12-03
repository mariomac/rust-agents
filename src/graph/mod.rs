// #![feature(trace_macros)]

// TODO: improve meaningful error messages
//      - if src and dst types don't match
//      - extract nodes aren't used as receivers
//      - load nodes aren't used as senders
//      - node function signatures
//      - any node left unconnected
// TODO: allow optional nodes and bypassing channels
// TODO: allow broadcasting output

#[macro_export]
macro_rules! graph {
    (@chain_connections $connect_src:ident -> $connect_dst:ident $(-> $rest:ident)+) => {
        graph!(@chain_connections $connect_src -> $connect_dst);
        graph!(@chain_connections $connect_dst $(-> $rest)+);
    };
    (@chain_connections $connect_src:ident -> $connect_dst:ident) => {
        $connect_src.out = Some($connect_dst.sender.clone());
    };

    {
        extract {
            $($enode_name:ident : $enode_func:path,)*
        }
        transform {
            $($tnode_name:ident : $tnode_func:path,)*
        }
        load {
            $($lnode_name:ident : $lnode_func:path,)*
        }
        $($connect_src:ident $(-> $connect_rest:ident)+ ,)*
    } => {

        struct Extractor<T: Send + 'static> {
            // TODO: check if there is a nicer way to define async function references
            func: fn(tokio_util::sync::CancellationToken, tokio::sync::mpsc::Sender<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
            out: Option<tokio::sync::mpsc::Sender<T>>,
        }

        struct Transformer<IT: Send + 'static, OT: Send + 'static> {
            func: fn(tokio_util::sync::CancellationToken, tokio::sync::mpsc::Receiver<IT>, tokio::sync::mpsc::Sender<OT>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,

            // the channel used to receive messages, and the sender that other nodes would use as out sender
            receiver: tokio::sync::mpsc::Receiver<IT>,
            sender: tokio::sync::mpsc::Sender<IT>,

            // the sender reference to other nodes, used to send stuff
            out : Option<tokio::sync::mpsc::Sender<OT>>,
        }

        struct Loader<T: Send + 'static> {
            func: fn(tokio_util::sync::CancellationToken, tokio::sync::mpsc::Receiver<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output=()> + Send>>,
            receiver: tokio::sync::mpsc::Receiver<T>,
            sender: tokio::sync::mpsc::Sender<T>,
        }

        // instantiate extractors, transformers, and loaders
        $(
            let mut $enode_name = Extractor {
                // TODO: check if there is a nicer way to refer to this
                func: |ct, tx| Box::pin($enode_func(ct, tx)),
                out: None,
            };
        )*
        $(
            let (s, r) = tokio::sync::mpsc::channel(10);
            let mut $tnode_name = Transformer {
                // TODO: check if there is a nicer way to refer to this
                func: |ct, rx, tx| Box::pin($tnode_func(ct, rx, tx)),
                receiver: r, sender: s, out: None,
            };
        )*
        $(
            let (s, r) = tokio::sync::mpsc::channel(10);
            let $lnode_name = Loader {
                // TODO: check if there is a nicer way to refer to this
                func: |ct, tx| Box::pin($lnode_func(ct, tx)),
                receiver: r, sender: s,
            };
        )*

        // decide the out channel for each node in the connection graph
        $(
            graph!(@chain_connections $connect_src $(-> $connect_rest)+);
        )*

        let ct = tokio_util::sync::CancellationToken::new();

        tokio::join!(
        $(
            // TODO: replace unwrap with proper error handling
            ($enode_name.func)(ct.clone(), $enode_name.out.unwrap()),
        )*
        $(
            ($tnode_name.func)(ct.clone(), $tnode_name.receiver, $tnode_name.out.unwrap()),
        )*
        $(
            ($lnode_name.func)(ct.clone(), $lnode_name.receiver),
        )*
        );
    };
}
