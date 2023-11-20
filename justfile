provision:
    cd tf && terraform init
    cd tf && terraform apply

deploy_website:
    aws s3 sync website s3://late-mate-website