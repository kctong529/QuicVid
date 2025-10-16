CC = gcc
CFLAGS = -I./quiche/quiche/include -L./quiche/target/release
LDFLAGS = ./quiche/target/release/libquiche.a -framework Security -framework Foundation -lresolv -lpthread -lm -mmacosx-version-min=14.5

quiche_client: quiche_client.c
	$(CC) $(CFLAGS) -o quiche_client quiche_client.c $(LDFLAGS)

clean:
	rm -f quiche_client

.PHONY: clean