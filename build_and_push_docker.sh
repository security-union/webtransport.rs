TAG=$(git rev-parse --short HEAD)

docker build -t securityunion/webtransport-leptos:$TAG .
docker push securityunion/webtransport-leptos:$TAG