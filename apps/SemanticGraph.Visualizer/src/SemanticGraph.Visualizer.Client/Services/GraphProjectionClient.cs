using System.Net.Http.Json;
using System.Text.Json;
using SemanticGraph.Visualizer.Client.Models;

namespace SemanticGraph.Visualizer.Client.Services;

public sealed class GraphProjectionClient
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);

    private readonly HttpClient _httpClient;
    private long _nextRequestId;

    public GraphProjectionClient(HttpClient httpClient)
    {
        _httpClient = httpClient;
    }

    public async Task<GraphProjectionDto> GetProjectionAsync(
        int limit,
        CancellationToken cancellationToken = default)
    {
        var request = new JsonRpcRequestDto<GraphProjectionParamsDto>(
            "2.0",
            Interlocked.Increment(ref _nextRequestId),
            "graph.projection",
            new GraphProjectionParamsDto(limit));

        using var response = await _httpClient.PostAsJsonAsync(
            "rpc",
            request,
            JsonOptions,
            cancellationToken);

        if (!response.IsSuccessStatusCode)
        {
            throw new GraphProjectionClientException(
                $"backend returned HTTP {(int)response.StatusCode}");
        }

        var rpcResponse = await response.Content.ReadFromJsonAsync<JsonRpcResponseDto<GraphProjectionDto>>(
            JsonOptions,
            cancellationToken);

        if (rpcResponse is null)
        {
            throw new GraphProjectionClientException("backend returned an empty JSON-RPC response");
        }

        if (rpcResponse.Error is not null)
        {
            throw new GraphProjectionClientException(
                $"JSON-RPC {rpcResponse.Error.Code}: {rpcResponse.Error.Message}");
        }

        return rpcResponse.Result
            ?? throw new GraphProjectionClientException("backend returned no graph projection result");
    }
}
