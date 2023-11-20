resource "aws_route53_zone" "late_mate_com" {
  name = "late-mate.com"
}

resource "aws_route53_record" "late_mate_com_caa" {
  name = "late-mate.com"
  type = "CAA"
  zone_id = aws_route53_zone.late_mate_com.id
  ttl = 600
  records = [
    "0 issue \"amazonaws.com\""
  ]
}

resource "aws_route53_record" "late_mate_com_acm_validation" {
  for_each = {
    for dvo in aws_acm_certificate.late_mate_com.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = aws_route53_zone.late_mate_com.zone_id
}

resource "aws_route53_record" "late_mate_com_web" {
  for_each = toset(["A", "AAAA"])

  name = "late-mate.com"
  type = each.key
  zone_id = aws_route53_zone.late_mate_com.id

  alias {
    evaluate_target_health = false
    name                   = aws_cloudfront_distribution.website.domain_name
    zone_id                = aws_cloudfront_distribution.website.hosted_zone_id
  }
}