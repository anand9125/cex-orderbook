-- Add migration script here
 CREATE TABLE users (   
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),  
                   email text NOT NULL UNIQUE,      
                        password text NOT NULL 
                            )