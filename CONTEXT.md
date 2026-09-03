# Domain Glossary

## API Account

A person or administrator identity that can sign in to the HTTP API and use protected Personal API, Web Admin, and Web Client capabilities. An API Account is not a RustDesk network connection credential.

## API Session

One authenticated login of an API Account from a client device. A session can expire or be revoked independently from other sessions belonging to the same account.

## RustDesk Peer

A RustDesk endpoint known to the ID/rendezvous server by its RustDesk ID, UUID, public key, and recent network registration. A Peer can exist and make remote connections without an API Account or API Session.

## API Device

The API-visible representation of a RustDesk Peer after an explicit, verifiable association has been established between the Peer and an API Account. A Peer is not automatically an API Device merely because an account used the same IP address or username.

## Address Book

An API Account's saved collection of RustDesk endpoints and display metadata. Address Book access is an HTTP API capability and does not grant or deny the underlying RustDesk network connection.

## User Group

A collection of API Accounts used to share API-visible resources and policies. It is distinct from a Device Group.

## Device Group

A collection of API Devices used for organization and access control. It does not alter the RustDesk rendezvous or relay protocol by itself.

## API Authentication

Proof that a request to the HTTP API belongs to an active API Session. API Authentication protects Personal API, Web Admin, and Web Client data.

## Connection Authorization

The rules applied by `hbbs` and `hbbr` when RustDesk clients register, rendezvous, punch holes, or relay traffic. In this project, Connection Authorization remains independent of API Authentication: clients that are not logged into the API may still make remote connections under the existing RustDesk `KEY` rules.
