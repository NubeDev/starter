FROM nats:2-alpine

LABEL org.opencontainers.image.title="nexus testing nats"
LABEL org.opencontainers.image.description="No-auth NATS broker for local Nexus tests"

EXPOSE 4222/tcp 8222/tcp 6222/tcp

CMD ["-m", "8222"]
