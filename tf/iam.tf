data "aws_iam_policy_document" "allow_s3_actions_on_the_website_bucket" {
  statement {
    actions = [
      "s3:*"
    ]
    resources = [
      aws_s3_bucket.website.arn,
      "${aws_s3_bucket.website.arn}/*"
    ]
  }
}

resource "aws_iam_user" "website_s3_manager" {
  name = "website_s3_manager"
}

resource "aws_iam_access_key" "website_s3_manager" {
  user = aws_iam_user.website_s3_manager.name
}

resource "aws_iam_user_policy" "allow_website_s3_manager_to_manage" {
  name = "allow_website_s3_manager_to_manage"
  user = aws_iam_user.website_s3_manager.name

  policy = data.aws_iam_policy_document.allow_s3_actions_on_the_website_bucket.json
}