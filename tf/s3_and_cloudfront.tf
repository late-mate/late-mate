resource "aws_s3_bucket" "website" {
  bucket = "late-mate-website"
}

data "aws_iam_policy_document" "website_allow_cf_access" {
  statement {
    principals {
      type        = "Service"
      identifiers = ["cloudfront.amazonaws.com"]
    }

    actions = [
      "s3:GetObject",
      "s3:ListBucket",
    ]

    resources = [
      aws_s3_bucket.website.arn,
      "${aws_s3_bucket.website.arn}/*",
    ]

    condition {
      test     = "StringEquals"
      variable = "AWS:SourceArn"
      values   = [aws_cloudfront_distribution.website.arn]
    }
  }
}

resource "aws_s3_bucket_policy" "website_allow_cf_access" {
  bucket = aws_s3_bucket.website.id
  policy = data.aws_iam_policy_document.website_allow_cf_access.json
}

resource "aws_cloudfront_origin_access_control" "website" {
  name                              = "late-mate-website-oac"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

resource "aws_cloudfront_cache_policy" "website" {
  name        = "late-mate-website-policy"
  default_ttl = 3
  max_ttl     = 31536000
  min_ttl     = 3

  parameters_in_cache_key_and_forwarded_to_origin {
    enable_accept_encoding_gzip = true
    enable_accept_encoding_brotli = true

    cookies_config {
      # it's all static S3 anyway, no cookies
      cookie_behavior = "none"
    }

    headers_config {
      # same
      header_behavior = "none"
    }

    query_strings_config {
      # handy to debug the cache
      query_string_behavior = "all"
    }
  }
}

resource "aws_acm_certificate" "late_mate_com" {
  # has to be in us-east-1 for CloudFront to use it
  provider = aws.us-east-1

  domain_name       = "late-mate.com"
  validation_method = "DNS"

  # otherwise I can't issue the cert
  depends_on = [aws_route53_record.late_mate_com_caa]
}

resource "aws_acm_certificate_validation" "late_mate_com" {
  provider = aws.us-east-1

  certificate_arn         = aws_acm_certificate.late_mate_com.arn
  validation_record_fqdns = [for record in aws_route53_record.late_mate_com_acm_validation : record.fqdn]
}

resource "aws_cloudfront_function" "use_index_html" {
  name    = "use-index-html"
  runtime = "cloudfront-js-1.0"
  code    = file("${path.module}/cloudfront_functions/use_index_html.js")
}

resource "aws_cloudfront_function" "strip_plau" {
  name    = "strip-plau"
  runtime = "cloudfront-js-1.0"
  code    = file("${path.module}/cloudfront_functions/strip_plau.js")
}

data "aws_cloudfront_cache_policy" "managed_caching_disabled" {
  name = "Managed-CachingDisabled"
}

data "aws_cloudfront_cache_policy" "managed_caching_optimized" {
  name = "Managed-CachingOptimized"
}

data "aws_cloudfront_origin_request_policy" "managed_ua_referer_headers" {
  name = "Managed-UserAgentRefererHeaders"
}

locals {
  s3_origin_id = "website-s3-origin"
  plausible_origin_id = "plausible-origin"
}

resource "aws_cloudfront_distribution" "website" {
  origin {
    origin_id                = local.s3_origin_id

    # apparently non-regional domain names are deprecated
    # https://stackoverflow.com/questions/65142577/is-cloudfront-origin-using-s3-global-domain-name-performing-better-than-regional
    domain_name              = aws_s3_bucket.website.bucket_regional_domain_name
    origin_access_control_id = aws_cloudfront_origin_access_control.website.id
  }

  origin {
    origin_id                = local.plausible_origin_id

    domain_name = "plausible.io"

    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  enabled             = true
  is_ipv6_enabled     = true
  http_version        = "http2and3"

  aliases = ["late-mate.com"]

  default_cache_behavior {
    allowed_methods  = ["GET", "HEAD", "OPTIONS"]
    cached_methods   = ["GET", "HEAD"]
    target_origin_id = local.s3_origin_id

    cache_policy_id = aws_cloudfront_cache_policy.website.id

    viewer_protocol_policy = "redirect-to-https"

    compress = true

    function_association {
      event_type   = "viewer-request"
      function_arn = aws_cloudfront_function.use_index_html.arn
    }
  }

  ordered_cache_behavior {
    path_pattern     = "/plau/js/script.*"

    allowed_methods  = ["GET", "HEAD"]
    cached_methods   = ["GET", "HEAD"]
    target_origin_id = local.plausible_origin_id

    cache_policy_id = data.aws_cloudfront_cache_policy.managed_caching_optimized.id

    viewer_protocol_policy = "https-only"

    compress = true

    function_association {
      event_type   = "viewer-request"
      function_arn = aws_cloudfront_function.strip_plau.arn
    }
  }

  ordered_cache_behavior {
    path_pattern     = "/plau/api/event"

    allowed_methods  = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
    cached_methods   = ["GET", "HEAD"]
    target_origin_id = local.plausible_origin_id

    cache_policy_id = data.aws_cloudfront_cache_policy.managed_caching_disabled.id

    origin_request_policy_id = data.aws_cloudfront_origin_request_policy.managed_ua_referer_headers.id

    viewer_protocol_policy = "https-only"

    compress = true

    function_association {
      event_type   = "viewer-request"
      function_arn = aws_cloudfront_function.strip_plau.arn
    }
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn = aws_acm_certificate_validation.late_mate_com.certificate_arn
    minimum_protocol_version = "TLSv1.2_2021"
    ssl_support_method = "sni-only"
  }

  custom_error_response {
    error_code = 404
    error_caching_min_ttl = 3
    response_code = 404
    response_page_path = "/404.html"
  }
}
