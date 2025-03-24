#[cfg(feature = "clap")]
use clap::Parser;
use kube::{
    Api, Client, CustomResourceExt, Result,
    api::{Patch, PatchParams, PostParams},
};
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "clap", command(author, version, about, long_about = None))]
pub struct OperatorArgs {
    #[cfg_attr(feature = "clap", arg(long, env = "CONTROLLER_NAME"))]
    pub controller_name: String,

    #[cfg_attr(feature = "clap", arg(long, env = "UPGRADE_CRDS"))]
    pub upgrade_crds: bool,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub async fn install_crd<K>(args: &OperatorArgs, client: &Client) -> Result<()>
where
    K: CustomResourceExt,
{
    let crd = <K as CustomResourceExt>::crd();
    let name = <K as CustomResourceExt>::crd_name();

    let api = Api::all(client.clone());
    if api.get_metadata_opt(name).await?.is_none() {
        let pp = PostParams {
            dry_run: false,
            field_manager: Some(args.controller_name.clone()),
        };
        api.create(&pp, &crd).await?;
        {
            #[cfg(feature = "tracing")]
            info!("created CRD: {name}");
        }
        Ok(())
    } else if args.upgrade_crds {
        let pp = PatchParams {
            dry_run: false,
            force: true,
            field_manager: Some(args.controller_name.clone()),
            field_validation: None,
        };
        api.patch(name, &pp, &Patch::Apply(&crd)).await?;
        {
            #[cfg(feature = "tracing")]
            info!("updated CRD: {name}");
        }
        Ok(())
    } else {
        {
            #[cfg(feature = "tracing")]
            info!("found CRD: {name}");
        }
        Ok(())
    }
}
