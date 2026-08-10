//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'external_api_key_created_response.g.dart';

/// ExternalApiKeyCreatedResponse
///
/// Properties:
/// * [applicationIds]
/// * [createdAt]
/// * [expiresAt]
/// * [id]
/// * [name]
/// * [status]
/// * [token]
@BuiltValue()
abstract class ExternalApiKeyCreatedResponse implements Built<ExternalApiKeyCreatedResponse, ExternalApiKeyCreatedResponseBuilder> {
  @BuiltValueField(wireName: r'application_ids')
  BuiltList<String> get applicationIds;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'expires_at')
  String? get expiresAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'token')
  String get token;

  ExternalApiKeyCreatedResponse._();

  factory ExternalApiKeyCreatedResponse([void updates(ExternalApiKeyCreatedResponseBuilder b)]) = _$ExternalApiKeyCreatedResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ExternalApiKeyCreatedResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ExternalApiKeyCreatedResponse> get serializer => _$ExternalApiKeyCreatedResponseSerializer();
}

class _$ExternalApiKeyCreatedResponseSerializer implements PrimitiveSerializer<ExternalApiKeyCreatedResponse> {
  @override
  final Iterable<Type> types = const [ExternalApiKeyCreatedResponse, _$ExternalApiKeyCreatedResponse];

  @override
  final String wireName = r'ExternalApiKeyCreatedResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ExternalApiKeyCreatedResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_ids';
    yield serializers.serialize(
      object.applicationIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.expiresAt != null) {
      yield r'expires_at';
      yield serializers.serialize(
        object.expiresAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'token';
    yield serializers.serialize(
      object.token,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ExternalApiKeyCreatedResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ExternalApiKeyCreatedResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.applicationIds.replace(valueDes);
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.expiresAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.token = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ExternalApiKeyCreatedResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ExternalApiKeyCreatedResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
