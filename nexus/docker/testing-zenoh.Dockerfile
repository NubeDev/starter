FROM eclipse/zenoh:latest

LABEL org.opencontainers.image.title="nexus testing zenoh"
LABEL org.opencontainers.image.description="No-auth Zenoh router for local Nexus tests"

EXPOSE 7447/tcp 8000/tcp
