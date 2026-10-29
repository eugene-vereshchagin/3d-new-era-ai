-- Nothing is sold any more: the project is free, and supporting it is a
-- donation that buys nothing. Every plan allows everything, and a limit
-- that is not written is no limit. The rows stay so a limit can come back
-- as data, without code, if the server ever needs one.
update plans set limits = '{}';
