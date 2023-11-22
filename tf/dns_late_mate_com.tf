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

// Mail stuff (points at Dan's Fastmail)

resource "aws_route53_record" "late_mate_com_mx" {
  for_each = toset(["late-mate.com", "*.late-mate.com"])

  name    = each.key
  type    = "MX"
  zone_id = aws_route53_zone.late_mate_com.id
  ttl = 600
  records = [
    "10 in1-smtp.messagingengine.com",
    "20 in2-smtp.messagingengine.com"
  ]
}

resource "aws_route53_record" "late_mate_com_txt" {
  name    = "late-mate.com"
  type    = "TXT"
  zone_id = aws_route53_zone.late_mate_com.id
  ttl = 600
  records = [
    # SPF record (servers that we allow to send email from @late-mate.com)
    # - messagingengine.com is Fastmail, mailgun is (probably?) buttondown
    # - Buttondown apparently uses a bunch of different ESPs, but doesn't rotate people
    #   to new ones after they validated DNS records in the settings.
    #   I asked to switch us to a ESP that supports sending from the top level domain,
    #   got switched to Postmark (yay) (author: Dan)
    "v=spf1 include:spf.messagingengine.com include:spf.mtasv.net ?all",
    # google search console verification
    "google-site-verification=9VNWCK8ztMYpCFAV0S549RsX1EDFil5nxOymIGR6ULM"
  ]
}

# fastmail sending mail from @late-mate.com
resource "aws_route53_record" "late_mate_com_dkim_cname" {
  for_each = {
    "fm1._domainkey.late-mate.com": "fm1.late-mate.com.dkim.fmhosted.com"
    "fm2._domainkey.late-mate.com": "fm2.late-mate.com.dkim.fmhosted.com"
    "fm3._domainkey.late-mate.com": "fm3.late-mate.com.dkim.fmhosted.com"
  }
  name    = each.key
  type    = "CNAME"
  zone_id = aws_route53_zone.late_mate_com.id
  ttl = 600
  records = [
    each.value
  ]
}

# buttondown sending email from @late-mate.com (via postmark)
resource "aws_route53_record" "late_mate_com_dkim_txt" {
  name    = "20231121212519pm._domainkey.late-mate.com"
  type    = "TXT"
  zone_id = aws_route53_zone.late_mate_com.id
  ttl = 600
  records = [
    "k=rsa;p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQCYYdqE/rvOLjkBqe2cI5hjX4hOsL66siYnDmlKrdWVlhAUx+Zls0JtgCMzbAqOPC41Eme0snX35rQvm5uz3qM5DPT3RqBt/Sn9LfTNn6Yn8yNTzLQAmyisLC63cEn+BUFpddCTvbITWLe0xxMV3quxyl1c4rseDJcgTLIHTaFgQwIDAQAB"
  ]
}

# psotmark's Return-Path domain (for bounced email) (used by Buttondown)
resource "aws_route53_record" "late_mate_com_return_path" {
  name    = "pm-bounces.late-mate.com"
  type    = "CNAME"
  zone_id = aws_route53_zone.late_mate_com.id
  ttl = 600
  records = [
    "pm.mtasv.net"
  ]
}

# NOTE: in aspf=r and adkim=r "r" stands for "relaxed"; it means that SPF and DKIM checks will pass for
#       subdomains. If they are "strict", foobar.late-mate.com will fail SPF/DKIM check
resource "aws_route53_record" "late_mate_com_dmarc" {
  name    = "_dmarc.late-mate.com"
  type    = "TXT"
  zone_id = aws_route53_zone.late_mate_com.id
  ttl = 600
  records = [
    "v=DMARC1; p=reject; pct=100; rua=mailto:re+bhx1qniybxj@dmarc.postmarkapp.com; ruf=mailto:dmarcfail@dgroshev.com; sp=none; adkim=r; aspf=r;"
  ]
}
