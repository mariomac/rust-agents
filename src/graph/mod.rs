// #![feature(trace_macros)]

// TODO: improve meaningful error messages
//      - if src and dst types don't match
//      - extract nodes aren't used as receivers
//      - load nodes aren't used as senders
//      - node function signatures
//      - any node left unconnected

pub use paste::paste;

#[macro_export]
macro_rules! graph {
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
        $($connect_src:ident -> $connect_dst:path,)*
    } => {
        fn check_enode_signature<T: Send + 'static>(
            _f: impl Fn(CancellationToken, tokio::sync::mpsc::Sender<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        ) {}

        $(
            const _: fn() = || {
                // This will fail to compile if $enode_func doesn't match the expected signature
                check_enode_signature(|ct, out| Box::pin($enode_func(ct, out)));
            };
        )*
        // TODO: validate tnode and lnode signatures

        ::paste::paste! {
            // create input channels for transform nodes
            $(
            let ([<$tnode_name _tx>], [<$tnode_name _rx>]) = tokio::sync::mpsc::channel(100);
            )*
            // create input channels for load nodes
            $(
            let ([<$lnode_name _tx>], [<$lnode_name _rx>]) = tokio::sync::mpsc::channel(100);
            )*
            // decide the output channel for each extract and transform node
            $(
            let [<$connect_src _outch>] = [<$connect_dst _tx>].clone();
            )*

            let ct = CancellationToken::new();
            tokio::join!(
            $(
                $enode_func(ct.clone(), [<$enode_name _outch>].clone()),
            )*
            $(
                $tnode_func(ct.clone(), [<$tnode_name _rx>], [<$tnode_name _outch>].clone()),
            )*
            $(
                $lnode_func(ct.clone(), [<$lnode_name _rx>]),
            )*
            );
        }

    };
}
