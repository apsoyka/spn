# Introduction

Save Page Now is a feature provided by the [Internet Archive](https://archive.org) that allows registered users to programatically create archival copies of web pages via an API.

This repository contains the code for a simple command-line interface for this API, and allows users to easily archive multiple pages at once.

# Authentication

First, retrieve your API keys from the page at [Internet Archive S3-Like API Keys](https://archive.org/account/s3.php). You will need these credentials in order to use the program; unauthenticated users are forbidden.

Then, either pass your credentials as program arguments, save them to a dotfile in the current working directory, or set them as environment variables.
