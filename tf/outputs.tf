output "late_mate_com_ns" {
  value = aws_route53_zone.late_mate_com.name_servers
}

output "website_s3_bucket_manager_access_key_id" {
  value = aws_iam_access_key.website_s3_manager.id
}

output "website_s3_bucket_manager_access_key_secret" {
  value = aws_iam_access_key.website_s3_manager.secret
  sensitive = true
}

output "website_s3_bucket_name" {
  value = aws_s3_bucket.website.id
}
