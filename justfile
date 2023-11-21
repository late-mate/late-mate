provision:
    cd tf && terraform init
    cd tf && terraform apply

deploy_website:
    aws s3 sync website s3://late-mate-website

gh_action_secrets: provision
    terraform output -state=tf/terraform.tfstate website_s3_bucket_manager_access_key_id
    terraform output -state=tf/terraform.tfstate website_s3_bucket_manager_access_key_secret
    terraform output -state=tf/terraform.tfstate website_s3_bucket_name