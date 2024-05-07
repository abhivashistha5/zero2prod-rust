-- migrations/20240507161720_create_subscription_table.sql

-- Create the subscriptions table
CREATE TABLE subscriptions(
  id uuid NOT NULL,
  PRIMARY KEY (id),
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  subscribed_at timestamptz NOT NULL
);
