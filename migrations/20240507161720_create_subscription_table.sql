-- migrations/20240507161720_create_subscription_table.sql

-- Create Subscription Table
CREATE TABLE subscriptions(
  id uuid NOT NULL,
  PRIMARY KEY (id),
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  subscribed_at timestamp NOT NULL
);
