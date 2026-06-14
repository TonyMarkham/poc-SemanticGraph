namespace SemanticGraph.Visualizer.Client.Services;

public sealed class GraphProjectionClientException : Exception
{
    public GraphProjectionClientException(string message)
        : base(message)
    {
    }
}
