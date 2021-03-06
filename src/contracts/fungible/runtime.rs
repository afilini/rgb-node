// RGB standard library
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use core::convert::TryFrom;
use std::path::PathBuf;

use lnpbp::TryService;

use super::{Config, IssueStructure, Processor, Request};
use crate::api::{self, Reply};
use crate::error::{ApiErrorType, BootstrapError, RuntimeError, ServiceError, ServiceErrorDomain};

use super::cache::{Cache, FileCache, FileCacheConfig};

pub struct Runtime {
    /// Original configuration object
    config: Config,

    /// Request-response API socket
    api_rep: zmq::Socket,

    /// Publish-subscribe API socket
    api_pub: zmq::Socket,

    /// Stash REQ API socket
    stash_req: zmq::Socket,

    /// Stash PUB API socket
    stash_sub: zmq::Socket,

    /// RGB fungible assets data cache: relational database sharing the client-
    /// friendly asset information with clients
    cacher: FileCache,

    processor: Processor,
}

impl Runtime {
    /// Internal function for avoiding index-implementation specific function
    /// use and reduce number of errors. Cacher may be switched with compile
    /// configuration options and, thus, we need to make sure that the structure
    /// we use corresponds to certain trait and not specific type.
    fn cache(&self) -> &impl Cache {
        &self.cacher
    }

    pub fn init(config: Config, context: &mut zmq::Context) -> Result<Self, BootstrapError> {
        let processor = Processor::new()?;

        let api_rep = context
            .socket(zmq::REP)
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;
        api_rep
            .connect(&config.rpc_endpoint.to_string())
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;

        let api_pub = context
            .socket(zmq::PUB)
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;
        api_pub
            .connect(&config.rpc_endpoint.to_string())
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;

        let stash_req = context
            .socket(zmq::REQ)
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;
        stash_req
            .connect(&config.rpc_endpoint.to_string())
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;

        let stash_sub = context
            .socket(zmq::SUB)
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;
        stash_sub
            .connect(&config.rpc_endpoint.to_string())
            .map_err(|e| BootstrapError::ZmqSocketError(e))?;

        let cacher = FileCache::new(FileCacheConfig {
            data_dir: PathBuf::from(&config.cache),
        })
        .map_err(|err| BootstrapError::Other)?;

        Ok(Self {
            config,
            api_rep,
            api_pub,
            stash_req,
            stash_sub,
            cacher,
            processor,
        })
    }
}

#[async_trait]
impl TryService for Runtime {
    type ErrorType = RuntimeError;

    async fn try_run_loop(mut self) -> Result<!, RuntimeError> {
        loop {
            match self.run().await {
                Ok(_) => debug!("API request processing complete"),
                Err(err) => {
                    error!("Error processing API request: {}", err);
                    Err(err)?;
                }
            }
        }
    }
}

impl Runtime {
    async fn run(&mut self) -> Result<(), RuntimeError> {
        /*
        let req: Multipart = self
            .api_rep
            .recv_multipart(0)
            .map_err(|err| RuntimeError::zmq_reply(&self.config.socket_rep, err))?
            .into_iter()
            .map(zmq::Message::from)
            .collect();
        trace!("New API request");

        trace!("Received API request {:x?}, processing ... ", req[0]);
        let reply = self
            .proc_request(req)
            .map_err(|domain| ServiceError::contract(domain, "fungible"))
            .inspect_err(|err| error!("Error processing request: {}", err))
            .await
            .unwrap_or_else(Reply::Failure);

        trace!(
            "Received response from command processor: `{}`; replying to client",
            reply
        );
        self.api_rep
            .send_multipart(Multipart::from(reply), 0)
            .map_err(|err| RuntimeError::zmq_reply(&self.config.socket_rep, err))?;
         */

        Ok(())
    }

    /*
    async fn proc_request(&mut self, req: Multipart) -> Result<(), ServiceErrorDomain> {
        let command = Request::try_from(req)?;

        let future = match command {
            Request::Issue(issue_request) => self.request_issue(issue_request),
            unknown_command => Err(ServiceErrorDomain::Api(ApiErrorType::UnimplementedCommand))?,
        };

        // TODO: Respond to client
        match future.await {
            Ok(_) => {}
            Err(err) => {}
        }

        Ok(())
    }
     */

    async fn request_issue(
        &mut self,
        issue: api::fungible::Issue,
    ) -> Result<(), ServiceErrorDomain> {
        debug!("Got ISSUE {}", issue);

        let issue_structure = match issue.inflatable {
            None => IssueStructure::SingleIssue,
            Some(seal_spec) => IssueStructure::MultipleIssues {
                max_supply: issue.supply.ok_or(ServiceErrorDomain::Api(
                    ApiErrorType::MissedArgument {
                        request: "Issue".to_string(),
                        argument: "supply".to_string(),
                    },
                ))?,
                reissue_control: seal_spec,
            },
        };

        let (asset, genesis) = self.processor.issue(
            self.config.network,
            issue.ticker,
            issue.title,
            issue.description,
            issue_structure,
            issue.allocate,
            issue.precision,
            vec![],
            issue.dust_limit,
        )?;

        // TODO: Save asset and genesis by sending a message to stashd
        self.cacher.add_asset(asset)?;

        // TODO: Send push request to client informing about cache update

        Ok(())
    }
}
