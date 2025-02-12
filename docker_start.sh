#! /bin/bash

docker compose --env-file cli/.env up -d --build jito-tip-router-ncn-keeper --remove-orphans