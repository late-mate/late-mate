terraform {
  backend "s3" {
    bucket = "late-mate-tfstate"
    key    = "main.tfstate"
    region = "eu-west-2"
  }
}

provider "aws" {
}

provider "aws" {
  region = "us-east-1"
  alias = "us-east-1"
}