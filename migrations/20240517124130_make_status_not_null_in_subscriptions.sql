BEGIN;
  UPDATE subscriptions
    SET status = 'CONFIRMED'
    WHERE status IS NULL;

  ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
