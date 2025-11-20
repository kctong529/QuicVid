CC = gcc
CFLAGS = -I./quiche/quiche/include
LDFLAGS = ./quiche/target/release/libquiche.a -lresolv -lpthread -lm

UNAME_S := $(shell uname -s)

ifeq ($(UNAME_S),Darwin)
    # macOS-specific flags
    LDFLAGS += -framework Security -framework Foundation -mmacosx-version-min=14.5
endif

# Default target: build both client and server
all: quiche_client quiche_server

quiche_client: quiche_client.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

quiche_server: quiche_server.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f quiche_client quiche_server

.PHONY: all clean
